use mongodb::Database;
use crate::models::book::Book;
use crate::models::book_segment::BookSegment;
use anyhow::Result;

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
    let book_result = book_collection.insert_one(book.clone())
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
        segment_collection.insert_many(final_segments)
            .session(&mut session)
            .await?;
    }

    // 3. Insert Chapters if present
    if !chapters.is_empty() {
        let chapter_collection = db.collection::<crate::models::book_chapter::BookChapter>("book_chapters");
        let mut final_chapters = chapters;
        for chap in &mut final_chapters {
            chap.book_id = book.id.expect("Book ID must be set");
        }
        chapter_collection.insert_many(final_chapters)
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

/// Chunks text into segments of roughly `chunk_size` words with `overlap` words.
pub fn chunk_text(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut chunks = Vec::new();

    if words.is_empty() {
        return chunks;
    }

    let mut start = 0;
    while start < words.len() {
        let end = (start + chunk_size).min(words.len()); //This calculates where the current chunk should end. It tries to take chunk_size number of words (start + chunk_size). However, if this exceeds the total number of words left (words.len()), the .min(words.len()) part ensures we don't go out of bounds and instead just grab whatever words are remaining.
        let chunk = words[start..end].join(" ");
        chunks.push(chunk);

        if end == words.len() {
            break;
        }

        // Advance by chunk_size minus overlap
        if start + chunk_size >= words.len() {
            break;
        }
        start += chunk_size - overlap;
    }

    chunks
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
