use axum::{
    extract::{State, Multipart},
    Json, 
    response::IntoResponse, 
    http::StatusCode
};
use mongodb::{Database, bson::DateTime};
use crate::models::book::Book;
use crate::models::book_segment::BookSegment;
use crate::services::{book_service, pdf_service, epub_service};

/// Returns true if the file bytes look like an EPUB (which is a ZIP archive starting with PK).
fn is_epub(data: &[u8]) -> bool {
    data.starts_with(b"PK")
}

pub async fn create_book(
    State(db): State<Database>,
    user: crate::middleware::auth::AuthenticatedUser,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut title = String::new();
    let mut author = None;
    let mut persona = None;
    let clerk_id = user.user_id; // Identity resolved via JWT middleware
    let mut file_data = Vec::new();

    // Parse multipart fields — log errors at warning level so we can diagnose issues
    loop {
        match multipart.next_field().await {
            Ok(Some(field)) => {
                let name = field.name().unwrap_or("").to_string();
                match name.as_str() {
                    "title" => title = field.text().await.unwrap_or_default(),
                    "author" => author = Some(field.text().await.unwrap_or_default()),
                    "persona" => persona = Some(field.text().await.unwrap_or_default()),
                    "file" => {
                        match field.bytes().await {
                            Ok(b) => file_data = b.to_vec(),
                            Err(e) => {
                                eprintln!("[create_book] Failed to read multipart file field: {:?}", e);
                                return (StatusCode::BAD_REQUEST, format!("Failed to read file: {:?}", e)).into_response();
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(None) => break, // no more fields
            Err(e) => {
                eprintln!("[create_book] Multipart parsing error: {:?}", e);
                return (StatusCode::BAD_REQUEST, format!("Multipart error: {:?}", e)).into_response();
            }
        }
    }

    eprintln!("[create_book] Received: title={:?}, file_size={}, epub={}", title, file_data.len(), is_epub(&file_data));
    if title.is_empty() || clerk_id.is_empty() || file_data.is_empty() {
        return (StatusCode::BAD_REQUEST, "Missing required fields").into_response();
    }

    // 1. Quota Check (High-Assurance)
    // match book_service::check_user_upload_quota(&db, &clerk_id).await {
    //     Ok(allowed) if !allowed => {
    //         return (StatusCode::FORBIDDEN, "Plan limit reached. Upgrade to upload more books.").into_response();
    //     }
    //     Err(e) => {
    //         eprintln!("Error checking quota: {}", e);
    //         return (StatusCode::INTERNAL_SERVER_ERROR, "Error validating account state").into_response();
    //     }
    //     _ => {} // Allowed
    // }

    // 2. High-assurance processing
    let base_slug = book_service::generate_slug(&title);
    let slug = match book_service::make_slug_unique_for_user(&db, &clerk_id, &base_slug).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error generating unique slug: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Error generating unique slug").into_response();
        }
    };
    
    // Initial storage metadata (will be populated with actual R2 data by the background worker)
    let file_url = String::new();
    let storage_key = String::new();

    let mut book = Book {
        id: Some(mongodb::bson::oid::ObjectId::new()), // Pre-generate ID for high-assurance tracking
        clerk_id: clerk_id.clone(),
        title,
        author,
        persona,
        slug: slug.clone(),
        file_url,
        storage_key,
        cover_url: None,
        cover_key: None,
        file_size: file_data.len() as i64,
        total_segments: 0,
        created_at: DateTime::now(),
        updated_at: DateTime::now(),
    };

    let book_for_response = book.clone();
    let db_clone = db.clone();
    let clerk_id_clone = clerk_id.clone();
    let epub = is_epub(&file_data);
    
    // Trigger high-assurance atomic background processing
    tokio::spawn(async move {
        // 1. Extract text based on file type
        let extraction_result = if epub {
            epub_service::extract_text_from_epub(&file_data)
        } else {
            pdf_service::extract_text_from_pdf(&file_data)
        };

        // 2. Generate / extract cover
        let cover_bytes: Option<Vec<u8>> = if epub {
            // EPUB cover is embedded in the file itself
            epub_service::extract_cover_from_epub(&file_data).map(|(bytes, _mime)| bytes)
        } else {
            // PDF: use pdftoppm to render page 1
            let temp_dir = tempfile::tempdir().ok();
            temp_dir.and_then(|dir| {
                let pdf_path = dir.path().join("input.pdf");
                let png_path = dir.path().join("cover.png");
                std::fs::write(&pdf_path, &file_data).ok()?;
                crate::services::image_service::generate_cover_from_pdf(&pdf_path, &png_path).ok()?;
                std::fs::read(&png_path).ok()
            })
        };

        match extraction_result {
            Ok(text) => {
                let chunk_size = std::env::var("BOOK_CHUNK_SIZE").unwrap_or("500".into()).parse().unwrap_or(500);
                let chunk_overlap = std::env::var("BOOK_CHUNK_OVERLAP").unwrap_or("50".into()).parse().unwrap_or(50);

                let chunks = book_service::chunk_text(&text, chunk_size, chunk_overlap);
                let total_segments = chunks.len() as i32;
                book.total_segments = total_segments;

                // Upload book file to Cloudflare R2 (correct extension)
                let ext = if epub { "epub" } else { "pdf" };
                let file_filename = format!("{}.{}", slug, ext);
                match crate::services::storage_service::upload_pdf_to_r2(&file_data, &file_filename).await {
                    Ok(file_url) => {
                        println!("Successfully uploaded {} to R2: {}", ext.to_uppercase(), file_url);
                        book.file_url = file_url;
                        book.storage_key = file_filename.clone();
                    },
                    Err(e) => {
                        eprintln!("Failed to upload {} to R2 for {}: {:?}", ext.to_uppercase(), slug, e);
                        return;
                    }
                }

                // Upload cover to R2 (if extracted/generated)
                if let Some(cover_data) = cover_bytes {
                    let cover_filename = format!("{}-cover.png", slug);
                    match crate::services::storage_service::upload_image_to_r2(&cover_data, &cover_filename).await {
                        Ok(cover_url) => {
                            println!("Successfully uploaded cover to R2: {}", cover_url);
                            book.cover_url = Some(cover_url);
                            book.cover_key = Some(cover_filename);
                        },
                        Err(e) => eprintln!("Failed to upload cover to R2 for {}: {:?}", slug, e),
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

                // Atomic ingestion: Insert Book + Segments
                match book_service::ingest_book_atomic(&db_clone, book, segments).await {
                    Ok(_) => println!("Successfully atomically processed book {} into {} segments.", slug, total_segments),
                    Err(e) => eprintln!("Failed atomic ingestion for {}: {}", slug, e),
                }
            },
            Err(e) => eprintln!("Failed to extract text for {}: {}. Background task aborted.", slug, e),
        }
    });

    (StatusCode::CREATED, Json(book_for_response)).into_response()
}

#[derive(serde::Serialize)]
pub struct MetadataResponse {
    pub title: Option<String>,
    pub author: Option<String>,
    pub cover_b64: Option<String>,
}

pub async fn extract_metadata(
    State(_db): State<Database>,
    _user: crate::middleware::auth::AuthenticatedUser,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut file_data = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            file_data = field.bytes().await.unwrap_or_default().to_vec();
        }
    }

    if file_data.is_empty() {
        return (StatusCode::BAD_REQUEST, "Missing file").into_response();
    }

    let epub = is_epub(&file_data);

    // Extract title & author
    let (title, author) = if epub {
        match epub_service::extract_metadata_from_epub(&file_data) {
            Ok(m) => (m.title, m.author),
            Err(e) => { eprintln!("EPUB metadata error: {}", e); (None, None) }
        }
    } else {
        match pdf_service::extract_metadata(&file_data) {
            Ok(m) => (m.title, m.author),
            Err(_) => (None, None),
        }
    };

    // Extract cover preview as base64
    let cover_b64 = if epub {
        epub_service::extract_cover_from_epub(&file_data).and_then(|(cover_bytes, _mime)| {
            use base64::{Engine as _, engine::general_purpose};
            Some(general_purpose::STANDARD.encode(&cover_bytes))
        })
    } else {
        let temp_dir = tempfile::tempdir().ok();
        temp_dir.and_then(|dir| {
            let pdf_path = dir.path().join("input.pdf");
            std::fs::write(&pdf_path, &file_data).ok()?;
            crate::services::image_service::generate_base64_cover(&pdf_path).ok()
        })
    };

    (StatusCode::OK, Json(MetadataResponse {
        title,
        author,
        cover_b64,
    })).into_response()
}
