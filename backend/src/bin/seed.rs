// src/bin/seed.rs
// Run with: cargo run --bin seed
//
// Fetches all 32 Gutenberg books, processes them, and stores
// EPUBs in R2 and segments in MongoDB. Safe to re-run —
// books already marked as `ready` are skipped automatically.

use backend::db;

use backend::services::gutenberg_service::{
    build_gutenberg_client, chunk_chapter, extract_chapters_from_epub,
    fetch_epub, GUTENBERG_CATALOGUE,
};
use backend::services::storage_service;
use mongodb::bson::doc;
use std::time::Duration;

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
            .unwrap_or_else(mongodb::bson::oid::ObjectId::new);

        if let Some(ref doc) = existing {
            let status = doc.get_str("processing_status").unwrap_or("unknown");
            if status == "ready" {
                println!("  ✓ Already processed — skipping\n");
                continue;
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
        match backend::services::storage_service::upload_bytes(&r2_key, &epub_bytes, "application/epub+zip").await {
            Ok(_) => println!("    Stored at: {r2_key}"),
            Err(e) => {
                println!("  ✗ R2 upload failed: {e}\n");
                continue;
            }
        }

        // ── Cover Image ──────────────────────────────────────────────
        let cover_url = Some(format!("https://www.gutenberg.org/cache/epub/{gutenberg_id}/pg{gutenberg_id}.cover.medium.jpg"));
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

        let mut all_segments: Vec<String> = Vec::new();
        for chapter_html in &chapters {
            let chunks = chunk_chapter(chapter_html, 500);
            all_segments.extend(chunks);
        }

        println!(
            "    {} chapters → {} segments",
            chapters.len(),
            all_segments.len()
        );

        // ── Save segments to MongoDB ──────────────────────────────────
        println!("  → Saving segments to MongoDB...");

        // Remove any stale segments from a previous failed run
        segments_col
            .delete_many(doc! { "book_id": book_id })
            .await
            .ok();

        let segment_docs: Vec<mongodb::bson::Document> = all_segments
            .iter()
            .enumerate()
            .map(|(idx, text)| {
                doc! {
                    "book_id": book_id,
                    "clerk_id": "public",
                    "gutenberg_id": gutenberg_id as i64,
                    "segment_index": idx as i64,
                    "content": text,
                    "word_count": text.split_whitespace().count() as i64,
                    "page_number": 0,
                }
            })
            .collect();

        if !segment_docs.is_empty() {
            if let Err(e) = segments_col.insert_many(segment_docs).await {
                println!("  ✗ MongoDB segment insert failed: {e}\n");
                continue;
            }
        }

        // ── Upsert book record as Ready ───────────────────────────────
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
            }
        };

        books_col
            .update_one(
                doc! { "_id": book_id },
                book_doc,
            )
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