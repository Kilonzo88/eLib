// src/bin/seed.rs
// Run with: cargo run --bin seed
//
// Fetches all 32 Gutenberg books, processes them, and stores
// EPUBs in R2 and embedded segments in MongoDB. Safe to re-run —
// books with ready, embedded segments are skipped; missing vectors are rebuilt.

use backend::db;
use backend::models::book_segment::BookSegment;
use backend::services::gemini_service;

use backend::services::gutenberg_service::{
    build_gutenberg_client, chunk_chapter, extract_chapters_from_epub,
    fetch_epub, GUTENBERG_CATALOGUE,
};
// Removed unused import
use mongodb::bson::doc;
use std::time::Duration;

async fn mark_ai_unavailable(
    books_col: &mongodb::Collection<mongodb::bson::Document>,
    book_id: mongodb::bson::oid::ObjectId,
) {
    if let Err(e) = books_col
        .update_one(
            doc! { "_id": book_id },
            doc! {
                "$set": {
                    "processing_error": "AI features are unavailable for this book right now. Please try again later.",
                    "updated_at": mongodb::bson::DateTime::now(),
                }
            },
        )
        .await
    {
        println!("  ✗ Failed to record embedding error: {e}");
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
            .unwrap_or_else(mongodb::bson::oid::ObjectId::new);

        if let Some(ref doc) = existing {
            let status = doc.get_str("processing_status").unwrap_or("unknown");
            if status == "ready" {
                let expected_segments = doc.get_i32("total_segments").unwrap_or(0).max(0) as u64;
                let embedded_segments = segments_col
                    .count_documents(doc! {
                        "book_id": book_id,
                        "embedding": { "$type": "array" },
                    })
                    .await
                    .unwrap_or(0);

                if expected_segments > 0 && embedded_segments >= expected_segments {
                    println!("  ✓ Already processed with embeddings — skipping\n");
                    continue;
                }

                println!("  → Rebuilding missing embeddings ({embedded_segments}/{expected_segments} segments ready)...");
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

        if all_segments.is_empty() {
            println!("  ✗ No text segments extracted; skipping book\n");
            continue;
        }

        println!("  → Generating vector embeddings...");
        let embeddings = match gemini_service::embed_texts(all_segments.clone()).await {
            Ok(embeddings)
                if embeddings.len() == all_segments.len()
                    && embeddings.iter().all(|embedding| embedding.len() == 768) =>
            {
                embeddings
            }
            Ok(embeddings) => {
                println!(
                    "  ✗ Embedding response mismatch: {} vectors for {} segments",
                    embeddings.len(),
                    all_segments.len()
                );
                mark_ai_unavailable(&books_col, book_id).await;
                continue;
            }
            Err(e) => {
                println!("  ✗ Embedding generation failed: {e}");
                mark_ai_unavailable(&books_col, book_id).await;
                continue;
            }
        };

        // ── Save segments to MongoDB ──────────────────────────────────
        println!("  → Saving segments to MongoDB...");

        let segment_docs = all_segments
            .iter()
            .zip(&embeddings)
            .enumerate()
            .map(|(idx, (text, embedding))| {
                mongodb::bson::to_document(&BookSegment {
                    id: None,
                    clerk_id: "public".to_string(),
                    book_id,
                    content: text.clone(),
                    segment_index: idx as i32,
                    page_number: 0,
                    word_count: text.split_whitespace().count() as i32,
                    embedding: Some(embedding.clone()),
                    created_at: Some(mongodb::bson::DateTime::now()),
                    updated_at: Some(mongodb::bson::DateTime::now()),
                })
            })
            .collect::<Result<Vec<_>, _>>();

        let segment_docs = match segment_docs {
            Ok(docs) => docs,
            Err(e) => {
                println!("  ✗ Segment serialization failed: {e}\n");
                mark_ai_unavailable(&books_col, book_id).await;
                continue;
            }
        };

        // Remove stale segments only after extraction and embedding both succeed.
        if let Err(e) = segments_col.delete_many(doc! { "book_id": book_id }).await {
            println!("  ✗ Stale segment cleanup failed: {e}\n");
            mark_ai_unavailable(&books_col, book_id).await;
            continue;
        }

        if !segment_docs.is_empty() {
            if let Err(e) = segments_col.insert_many(segment_docs).await {
                println!("  ✗ MongoDB segment insert failed: {e}\n");
                mark_ai_unavailable(&books_col, book_id).await;
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
            },
            "$unset": {
                "processing_error": "",
            },
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