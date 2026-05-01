use axum::{
    extract::{State, Multipart},
    Json, 
    response::IntoResponse, 
    http::StatusCode
};
use mongodb::{Database, bson::DateTime};
use crate::models::book::Book;
use crate::models::book_segment::BookSegment;
use crate::services::{book_service, pdf_service};

pub async fn create_book(
    State(db): State<Database>,
    user: crate::middleware::auth::AuthenticatedUser,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut title = String::new();
    let mut author = None;
    let mut persona = None;
    let clerk_id = user.user_id; // Identity resolved via JWT middleware
    let mut pdf_data = Vec::new();

    // Parse multipart fields
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "title" => title = field.text().await.unwrap_or_default(),
            "author" => author = Some(field.text().await.unwrap_or_default()),
            "persona" => persona = Some(field.text().await.unwrap_or_default()),
            "file" => {
                pdf_data = field.bytes().await.unwrap_or_default().to_vec();
            }
            _ => continue,
        }
    }

    if title.is_empty() || clerk_id.is_empty() || pdf_data.is_empty() {
        return (StatusCode::BAD_REQUEST, "Missing required fields").into_response();
    }

    // 1. Quota Check (High-Assurance)
    match book_service::check_user_upload_quota(&db, &clerk_id).await {
        Ok(allowed) if !allowed => {
            return (StatusCode::FORBIDDEN, "Plan limit reached. Upgrade to upload more books.").into_response();
        }
        Err(e) => {
            eprintln!("Error checking quota: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Error validating account state").into_response();
        }
        _ => {} // Allowed
    }

    // 2. High-assurance processing
    let base_slug = book_service::generate_slug(&title);
    let slug = match book_service::make_slug_unique_for_user(&db, &clerk_id, &base_slug).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error generating unique slug: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Error generating unique slug").into_response();
        }
    };
    
    // Placeholder for storage (Vercel Blob)
    // (TODO) Implement actual storage upload in storage_service.rs
    let file_url = format!("https://placeholder.com/{}.pdf", slug);
    let file_blob_key = format!("blob_{}", slug);

    let mut book = Book {
        id: Some(mongodb::bson::oid::ObjectId::new()), // Pre-generate ID for high-assurance tracking
        clerk_id: clerk_id.clone(),
        title,
        author,
        persona,
        slug: slug.clone(),
        file_url,
        file_blob_key,
        cover_url: None,
        cover_blob_key: None,
        file_size: pdf_data.len() as i64,
        total_segments: 0,
        created_at: DateTime::now(),
        updated_at: DateTime::now(),
    };

    let book_for_response = book.clone();
    let db_clone = db.clone();
    let clerk_id_clone = clerk_id.clone();
    
    // Trigger high-assurance atomic background processing
    tokio::spawn(async move {
        // 1. Save PDF to temporary file for processing
        let temp_dir = match tempfile::tempdir() {
            Ok(dir) => dir,
            Err(e) => {
                eprintln!("Failed to create temp dir: {}", e);
                return;
            }
        };
        let pdf_path = temp_dir.path().join("input.pdf");
        let png_path = temp_dir.path().join("cover.png");
        
        if let Err(e) = std::fs::write(&pdf_path, &pdf_data) {
            eprintln!("Failed to write temp PDF: {}", e);
            return;
        }

        // 2. Process: Extract Text & Generate Cover
        let extraction_result = pdf_service::extract_text_from_pdf(&pdf_data);
        let cover_result = crate::services::image_service::generate_cover_from_pdf(&pdf_path, &png_path);

        match extraction_result {
            Ok(text) => {
                let chunks = book_service::chunk_text(&text, 500, 50);
                let total_segments = chunks.len() as i32;
                book.total_segments = total_segments;

                // 3. Upload to Cloud (Vercel Blob)
                // Upload PDF
                let pdf_filename = format!("books/{}.pdf", slug);
                match crate::services::storage_service::upload_to_blob(&pdf_filename, pdf_data).await {
                    Ok(pdf_blob) => {
                        book.file_url = pdf_blob.url;
                        book.file_blob_key = pdf_blob.pathname;
                    },
                    Err(e) => {
                        eprintln!("Failed to upload PDF to blob for {}: {}", slug, e);
                        return;
                    }
                }

                // Upload Cover (if generated)
                if cover_result.is_ok() {
                    let cover_filename = format!("covers/{}.png", slug);
                    if let Ok(cover_data) = std::fs::read(&png_path) {
                        if let Ok(cover_blob) = crate::services::storage_service::upload_to_blob(&cover_filename, cover_data).await {
                            book.cover_url = Some(cover_blob.url);
                            book.cover_blob_key = Some(cover_blob.pathname);
                        }
                    }
                }

                let segments: Vec<BookSegment> = chunks.into_iter().enumerate().map(|(i, content)| {
                    let word_count = content.split_whitespace().count() as i32;
                    BookSegment {
                        id: None,
                        clerk_id: clerk_id_clone.clone(),
                        book_id: book.id.expect("ID was pre-set"),
                        content,
                        segment_index: i as i32,
                        page_number: 0,
                        word_count,
                        created_at: Some(DateTime::now()),
                        updated_at: Some(DateTime::now()),
                    }
                }).collect();

                // Call our atomic ingestion helper (Transaction: Insert Book + Insert Segments)
                match book_service::ingest_book_atomic(&db_clone, book, segments).await {
                    Ok(_) => println!("Successfully atomically processed book {} into {} segments.", slug, total_segments),
                    Err(e) => eprintln!("Failed atomic ingestion for {}: {}", slug, e),
                }
            },
            Err(e) => eprintln!("Failed to process PDF text for {}: {}. Background task aborted.", slug, e),
        }
    });

    (StatusCode::CREATED, Json(book_for_response)).into_response()
}
