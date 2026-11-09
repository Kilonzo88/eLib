use crate::models::book::Book;
use crate::models::book_segment::BookSegment;
use anyhow::Result;
use mongodb::{
    Database,
    bson::{DateTime, doc, oid::ObjectId},
};

/// A high-assurance atomic operation that saves a book and all its segments to MongoDB.
/// If any part fails, the entire operation is rolled back.
pub async fn ingest_book_atomic(
    db: &Database,
    mut book: Book,
    segments: Vec<BookSegment>,
    chapters: Vec<crate::models::book_chapter::BookChapter>,
) -> Result<Book> {
    let client = db.client();
    let mut session = client.start_session().await?;

    session.start_transaction().await?; //Activated the staging mode until our actions are complete

    let book_collection = db.collection::<Book>("books");
    let segment_collection = db.collection::<BookSegment>("book_segments");

    // 1. Insert Book
    let book_result = book_collection
        .insert_one(book.clone())
        .session(&mut session)
        .await?;
    if let Some(id) = book_result.inserted_id.as_object_id() {
        book.id = Some(id);
    }

    // 2. Insert Segments (mapped with the new book ID)
    if !segments.is_empty() {
        let mut final_segments = segments;
        for seg in &mut final_segments {
            seg.book_id = book.id.expect("Book ID must be set");
        }
        segment_collection
            .insert_many(final_segments)
            .session(&mut session)
            .await?;
    }

    // 3. Insert Chapters if present
    if !chapters.is_empty() {
        let chapter_collection =
            db.collection::<crate::models::book_chapter::BookChapter>("book_chapters");
        let mut final_chapters = chapters;
        for chap in &mut final_chapters {
            chap.book_id = book.id.expect("Book ID must be set");
        }
        chapter_collection
            .insert_many(final_chapters)
            .session(&mut session)
            .await?;
    }

    // 4. Commit
    session.commit_transaction().await?;

    Ok(book)
}

/// Generates a URL-friendly slug from a string (e.g., "Hello World!" -> "hello-world")
pub fn generate_slug(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '-')
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}

/// Ensures the given base_slug is unique for the specified clerk_id.
/// If it exists, appends an incrementing suffix (-1, -2, etc.).
pub async fn make_slug_unique_for_user(
    db: &Database,
    clerk_id: &str,
    base_slug: &str,
) -> Result<String> {
    let collection = db.collection::<Book>("books");
    let mut slug = base_slug.to_string();
    let mut suffix = 1;

    loop {
        let filter = mongodb::bson::doc! { "clerk_id": clerk_id, "slug": &slug };
        // If no document is found, the slug is unique
        if collection.count_documents(filter).await? == 0 {
            break;
        }
        slug = format!("{}-{}", base_slug, suffix);
        suffix += 1;
    }

    Ok(slug)
}

/// Chunks text at full stops, targeting `chunk_size` words per segment.
pub fn chunk_text(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    if chunk_size == 0 {
        return Vec::new();
    }

    let sentences: Vec<String> = text
        .split_inclusive('.')
        .map(str::trim)
        .filter(|sentence| !sentence.is_empty())
        .map(str::to_owned)
        .collect();
    let mut chunks = Vec::new();
    let mut current: Vec<String> = Vec::new();
    let mut current_words = 0;

    for sentence in sentences {
        let sentence_words = sentence.split_whitespace().count();
        if !current.is_empty() && current_words + sentence_words > chunk_size {
            chunks.push(current.join(" "));

            // Carry only complete trailing sentences into the next chunk.
            let mut carried = Vec::new();
            let mut carried_words = 0;
            for previous in current.iter().rev() {
                let words = previous.split_whitespace().count();
                if carried_words + words > overlap {
                    break;
                }
                carried.insert(0, previous.clone());
                carried_words += words;
            }
            current = carried;
            current_words = carried_words;
        }
        current_words += sentence_words;
        current.push(sentence);
    }

    if !current.is_empty() {
        chunks.push(current.join(" "));
    }
    chunks
}

#[derive(Clone, Debug)]
pub struct SourceText {
    pub text: String,
    pub page_start: Option<i32>,
    pub page_end: Option<i32>,
    pub chapter_index: Option<i32>,
    pub chapter_title: Option<String>,
}

#[derive(Clone, Debug)]
pub struct LocatedTextChunk {
    pub content: String,
    pub page_start: Option<i32>,
    pub page_end: Option<i32>,
    pub chapter_index: Option<i32>,
    pub chapter_title: Option<String>,
}

pub fn chunk_source_texts(
    sources: Vec<SourceText>,
    chunk_size: usize,
    overlap: usize,
) -> Vec<LocatedTextChunk> {
    if chunk_size == 0 {
        return Vec::new();
    }

    let overlap = overlap.min(chunk_size - 1);
    let mut located_chunks = Vec::new();

    for source in sources {
        for content in chunk_text(&source.text, chunk_size, overlap) {
            located_chunks.push(LocatedTextChunk {
                content,
                page_start: source.page_start,
                page_end: source.page_end,
                chapter_index: source.chapter_index,
                chapter_title: source.chapter_title.clone(),
            });
        }
    }

    located_chunks
}

pub async fn check_user_upload_quota(db: &Database, clerk_id: &str) -> Result<bool> {
    let collection = db.collection::<Book>("books");
    let filter = mongodb::bson::doc! { "clerk_id": clerk_id };

    // Count existing books for this user
    let count = collection.count_documents(filter).await?;

    // (TODO) In a real app, we would fetch the user's tier from Clerk or a Users collection.
    // For now, we enforce a strict "100 Book" limit for development.
    let limit = 100;

    Ok(count < limit)
}

pub async fn mark_book_failed(db: &Database, book_id: ObjectId, reason: &str) -> Result<()> {
    let collection = db.collection::<Book>("books");
    let summary = reason.trim();
    let error_summary = if summary.len() > 500 {
        &summary[..500]
    } else {
        summary
    };

    collection
        .update_one(
            doc! { "_id": book_id },
            doc! {
                "$set": {
                    "processing_status": "failed",
                    "processing_error": error_summary,
                    "updated_at": DateTime::now(),
                }
            },
        )
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{SourceText, chunk_source_texts};

    #[test]
    fn keeps_source_location_on_each_chunk() {
        let chunks = chunk_source_texts(
            vec![SourceText {
                text: "one two three. four five six.".to_string(),
                page_start: Some(12),
                page_end: Some(13),
                chapter_index: None,
                chapter_title: None,
            }],
            3,
            0,
        );

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].page_start, Some(12));
        assert_eq!(chunks[0].page_end, Some(13));
    }

    #[test]
    fn keeps_chapter_location_on_each_chunk() {
        let chunks = chunk_source_texts(
            vec![SourceText {
                text: "one two. three four.".to_string(),
                page_start: None,
                page_end: None,
                chapter_index: Some(2),
                chapter_title: Some("Chapter Two".to_string()),
            }],
            2,
            0,
        );

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[1].chapter_index, Some(2));
        assert_eq!(chunks[1].chapter_title.as_deref(), Some("Chapter Two"));
    }
}
