// src/bin/seed.rs
// Run with: cargo run --bin seed
//
// Fetches all 32 Gutenberg books, processes them, and stores
// EPUBs in R2 and text segments in MongoDB. Safe to re-run —
// books with a ready, complete segment set are skipped; missing segments are rebuilt.
//
// Embeddings are not generated here. The `vector_index` autoEmbed index on
// `book_segments.content` makes Atlas embed new or updated segments with Voyage `voyage-4`.

use backend::db;
use backend::models::book_segment::BookSegment;
use backend::services::book_service::{SourceText, chunk_source_texts, mark_book_failed};

use backend::services::gutenberg_service::{
    GUTENBERG_CATALOGUE, build_gutenberg_client, extract_chapters_from_epub, fetch_epub,
};
use mongodb::bson::{doc, oid::ObjectId};
use std::time::Duration;

async fn mark_failed(database: &mongodb::Database, book_id: ObjectId, reason: &str) {
    if let Err(e) = mark_book_failed(database, book_id, reason).await {
        println!("  ✗ Failed to persist failure state: {e}");
    }
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    println!("═══════════════════════════════════════");
    println!("  eLib — Gutenberg Pre-processing Seed  ");
    println!("═══════════════════════════════════════\n");

    let database = db::connect().await;
    let client = build_gutenberg_client();

    let books_col = database.collection::<mongodb::bson::Document>("books");
    let segments_col = database.collection::<mongodb::bson::Document>("book_segments");

    let total = GUTENBERG_CATALOGUE.len();

    for (i, &(gutenberg_id, title, author)) in GUTENBERG_CATALOGUE.iter().enumerate() {
        println!("[{}/{}] {} — {}", i + 1, total, title, author);

        // ── Skip if already processed ─────────────────────────────────
        let existing = books_col
            .find_one(doc! { "gutenberg_id": gutenberg_id as i64 })
            .await
            .unwrap_or(None);

        let book_id = existing
            .as_ref()
            .and_then(|d| d.get_object_id("_id").ok())
            .unwrap_or_else(ObjectId::new);

        if let Some(ref doc) = existing {
            let status = doc.get_str("processing_status").unwrap_or("unknown");
            if status == "ready" {
                let expected_segments = doc.get_i32("total_segments").unwrap_or(0).max(0) as u64;
                let present_segments = segments_col
                    .count_documents(doc! { "book_id": book_id })
                    .await
                    .unwrap_or(0);

                if expected_segments > 0 && present_segments >= expected_segments {
                    println!("  ✓ Already processed — skipping\n");
                    continue;
                }

                println!(
                    "  → Rebuilding missing segments ({present_segments}/{expected_segments} present)..."
                );
            }
        }

        // ── Fetch EPUB ────────────────────────────────────────────────
        println!("  → Fetching EPUB from Gutenberg...");
        let epub_bytes = match fetch_epub(gutenberg_id, &client).await {
            Ok(b) => b,
            Err(e) => {
                println!("  ✗ Fetch failed: {e}\n");
                continue;
            }
        };
        println!("    Size: {:.1} KB", epub_bytes.len() as f64 / 1024.0);

        // ── Upload raw EPUB to R2 ─────────────────────────────────────
        println!("  → Uploading EPUB to R2...");
        let r2_key = format!("gutenberg/{gutenberg_id}/book.epub");
        match backend::services::storage_service::upload_bytes(
            &r2_key,
            &epub_bytes,
            "application/epub+zip",
        )
        .await
        {
            Ok(_) => println!("    Stored at: {r2_key}"),
            Err(e) => {
                println!("  ✗ R2 upload failed: {e}\n");
                continue;
            }
        }

        // ── Cover Image ──────────────────────────────────────────────
        let cover_url = Some(format!(
            "https://www.gutenberg.org/cache/epub/{gutenberg_id}/pg{gutenberg_id}.cover.medium.jpg"
        ));
        let cover_key: Option<String> = None;

        // ── Parse EPUB and chunk ──────────────────────────────────────
        println!("  → Parsing and chunking EPUB...");
        let chapters = match extract_chapters_from_epub(&epub_bytes) {
            Ok(c) => c,
            Err(e) => {
                println!("  ✗ EPUB parse failed: {e}\n");
                continue;
            }
        };

        let chunk_size = std::env::var("BOOK_CHUNK_SIZE")
            .unwrap_or("500".into())
            .parse()
            .unwrap_or(500);
        let chunk_overlap = std::env::var("BOOK_CHUNK_OVERLAP")
            .unwrap_or("50".into())
            .parse()
            .unwrap_or(50);
        let source_texts = chapters
            .iter()
            .enumerate()
            .map(|(index, chapter_html)| SourceText {
                text: html2text::from_read(chapter_html.as_bytes(), usize::MAX).unwrap_or_default(),
                page_start: None,
                page_end: None,
                chapter_index: Some(index as i32 + 1),
                chapter_title: None,
            })
            .collect();
        let all_segments = chunk_source_texts(source_texts, chunk_size, chunk_overlap);

        println!(
            "    {} chapters → {} segments",
            chapters.len(),
            all_segments.len()
        );

        if all_segments.is_empty() {
            println!("  ✗ No text segments extracted; skipping book\n");
            continue;
        }

        // ── Save segments to MongoDB ──────────────────────────────────
        println!("  → Saving segments to MongoDB...");

        let segment_docs = all_segments
            .iter()
            .enumerate()
            .map(|(idx, chunk)| {
                mongodb::bson::to_document(&BookSegment {
                    id: None,
                    clerk_id: "public".to_string(),
                    book_id,
                    content: chunk.content.clone(),
                    segment_index: idx as i32,
                    page_number: 0,
                    page_start: chunk.page_start,
                    page_end: chunk.page_end,
                    chapter_index: chunk.chapter_index,
                    chapter_title: chunk.chapter_title.clone(),
                    word_count: chunk.content.split_whitespace().count() as i32,
                    created_at: Some(mongodb::bson::DateTime::now()),
                    updated_at: Some(mongodb::bson::DateTime::now()),
                })
            })
            .collect::<Result<Vec<_>, _>>();

        let segment_docs = match segment_docs {
            Ok(docs) => docs,
            Err(e) => {
                println!("  ✗ Segment serialization failed: {e}\n");
                mark_failed(
                    &database,
                    book_id,
                    &format!("Segment serialization failed: {e}"),
                )
                .await;
                continue;
            }
        };

        // Remove stale segments only after extraction succeeds.
        if let Err(e) = segments_col.delete_many(doc! { "book_id": book_id }).await {
            println!("  ✗ Stale segment cleanup failed: {e}\n");
            mark_failed(
                &database,
                book_id,
                &format!("Stale segment cleanup failed: {e}"),
            )
            .await;
            continue;
        }

        if !segment_docs.is_empty() {
            if let Err(e) = segments_col.insert_many(segment_docs).await {
                println!("  ✗ MongoDB segment insert failed: {e}\n");
                mark_failed(&database, book_id, &format!("Segment insert failed: {e}")).await;
                continue;
            }
        }

        // ── Upsert book record as Ready ───────────────────────────────
        // Atlas embeds the inserted `content` values in the background once the
        // autoEmbed index is READY, so there is no embedding call to wait for here.
        let book_doc = doc! {
            "$set": {
                "gutenberg_id": gutenberg_id as i64,
                "title": title,
                "author": author,
                "persona": mongodb::bson::Bson::Null,
                "clerk_id": "public",
                "slug": format!("gutenberg-{}", gutenberg_id),
                "file_url": "",
                "storage_key": &r2_key,
                "cover_url": cover_url,
                "cover_key": cover_key,
                "file_size": epub_bytes.len() as i64,
                "total_segments": all_segments.len() as i32,
                "processing_status": "ready",
                "updated_at": mongodb::bson::DateTime::now(),
            },
            "$setOnInsert": {
                "_id": book_id,
                "created_at": mongodb::bson::DateTime::now(),
            },
            "$unset": {
                "processing_error": "",
            },
        };

        books_col
            .update_one(doc! { "_id": book_id }, book_doc)
            .with_options(
                mongodb::options::UpdateOptions::builder()
                    .upsert(true)
                    .build(),
            )
            .await
            .ok();

        println!("  ✓ Done\n");

        // Be polite to Gutenberg between books — 2 second pause
        if i < total - 1 {
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }

    println!("═══════════════════════════════════");
    println!("  Seeding complete.");
    println!("═══════════════════════════════════");
}
