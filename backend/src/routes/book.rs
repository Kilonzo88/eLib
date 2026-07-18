use axum::{
    extract::{State, Multipart, Path},
    Json, 
    response::IntoResponse, 
    http::StatusCode
};
use mongodb::bson::doc;
use mongodb::bson::oid::ObjectId;
use mongodb::{Database, bson::DateTime};
use serde::Deserialize;
use crate::models::book::Book;
use crate::models::book_segment::BookSegment;
use crate::models::book_chapter::BookChapter;
use crate::services::{book_service, pdf_service, epub_service};

/// Returns true if the file bytes look like an EPUB (which is a ZIP archive starting with PK).
fn is_epub(data: &[u8]) -> bool {
    data.starts_with(b"PK")
}

pub async fn create_book(
    State(db): State<Database>,
    user: crate::middleware::auth::OptionalUser,
    mut multipart: Multipart,
) -> impl IntoResponse {
    println!("[create_book] Handler reached. Clerk ID: {:?}", user.user_id);
    let mut title = String::new();
    let mut author = None;
    let mut persona = None;
    let clerk_id = user.user_id.unwrap_or_else(|| "anonymous".to_string()); // Identity resolved via OptionalUser extractor
    let mut file_temp_path: Option<tempfile::TempPath> = None;
    let mut file_size: usize = 0;

    // Parse multipart fields — log errors at warning level so we can diagnose issues
    loop {
        match multipart.next_field().await {
            Ok(Some(mut field)) => {
                let name = field.name().unwrap_or("").to_string();
                match name.as_str() {
                    "title" => {
                        let raw_title = field.text().await.unwrap_or_default();
                        // Sanitize null bytes which can happen with certain browser encodings or mobile proxies
                        title = raw_title.replace('\0', "");
                    }
                    "author" => author = Some(field.text().await.unwrap_or_default()),
                    "persona" => persona = Some(field.text().await.unwrap_or_default()),
                    "file" => {
                        let tf = match tempfile::NamedTempFile::new() {
                            Ok(tf) => tf,
                            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create temp file: {:?}", e)).into_response(),
                        };
                        let (file, path) = tf.into_parts();
                        let mut tokio_file = tokio::fs::File::from_std(file);
                        
                        while let Ok(Some(chunk)) = field.chunk().await {
                            if let Err(e) = tokio::io::AsyncWriteExt::write_all(&mut tokio_file, &chunk).await {
                                return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to write chunk: {:?}", e)).into_response();
                            }
                            file_size += chunk.len();
                        }
                        file_temp_path = Some(path);
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

    eprintln!("[create_book] Received: title={:?}, file_size={}", title, file_size);
    if title.is_empty() || clerk_id.is_empty() || file_temp_path.is_none() || file_size == 0 {
        return (StatusCode::BAD_REQUEST, "Missing required fields or empty file").into_response();
    }

    let file_temp_path = file_temp_path.unwrap();
    
    let epub = {
        let mut magic = [0u8; 2];
        if let Ok(mut f) = std::fs::File::open(&file_temp_path) {
            let _ = std::io::Read::read_exact(&mut f, &mut magic);
        }
        &magic == b"PK"
    };

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

    let book = Book {
        id: None, // Let MongoDB generate the _id to avoid serde type mismatch
        clerk_id: clerk_id.clone(),
        title,
        author,
        persona,
        slug: slug.clone(),
        file_url,
        storage_key,
        cover_url: None,
        cover_key: None,
        file_size: file_size as i64,
        total_segments: 0,
        gutenberg_id: None,
        processing_status: Some("processing".to_string()),
        created_at: DateTime::now(),
        updated_at: DateTime::now(),
    };

    // Insert book synchronously so frontend can navigate to it immediately
    let books_coll = db.collection::<Book>("books");
    let insert_result = match books_coll.insert_one(book.clone()).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[create_book] Failed to insert book {}: {}", slug, e);
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save book: {}", e)).into_response();
        }
    };
    // Use the MongoDB-generated _id (guaranteed to be an ObjectId)
    let book_id = insert_result.inserted_id.as_object_id().expect("MongoDB always generates ObjectId for _id");

    let mut book_for_response = book.clone();
    book_for_response.id = Some(book_id);
    let db_clone = db.clone();
    let clerk_id_clone = clerk_id.clone();
    let slug_clone = slug.clone();

    // Background task: text extraction, R2 upload, segments & chapters
    tokio::spawn(async move {
        println!("[create_book] Starting background processing for slug: {}", slug_clone);
        // CRITICAL: Keep file_temp_path (TempPath) alive for the entire background task.
        let _temp_guard = file_temp_path;
        let file_path_for_extract = _temp_guard.to_path_buf();
        let file_path_for_upload = _temp_guard.to_path_buf();
        let file_path_for_chapters = _temp_guard.to_path_buf();
        let result: Result<(), String> = async {
            println!("[create_book] Extracting text and cover...");
            let slug_for_blocking = slug_clone.clone();
            // 1. Heavy Extraction (Blocking)
            let (extraction_result, cover_bytes) = tokio::task::spawn_blocking(move || {
                let text_res = if epub {
                    epub_service::extract_text_from_epub(&file_path_for_extract)
                } else {
                    pdf_service::extract_text_from_pdf(&file_path_for_extract)
                };

                let cover = if epub {
                    match epub_service::extract_cover_from_epub(&file_path_for_extract) {
                        Some((bytes, _mime)) => Some(bytes),
                        None => {
                            eprintln!("[create_book] No cover found in EPUB for {}", slug_for_blocking);
                            None
                        }
                    }
                } else {
                    let temp_dir = tempfile::tempdir().ok();
                    let res = temp_dir.and_then(|dir| {
                        let png_path = dir.path().join("cover.png");
                        if let Err(e) = crate::services::image_service::generate_cover_from_pdf(&file_path_for_extract, &png_path) {
                            eprintln!("[create_book] PDF cover generation failed for {}: {:?}", slug_for_blocking, e);
                            return None;
                        }
                        std::fs::read(&png_path).ok()
                    });
                    if res.is_none() {
                        eprintln!("[create_book] PDF cover read failed for {} (check image_service logs)", slug_for_blocking);
                    }
                    res
                };
                (text_res, cover)
            }).await.map_err(|e| format!("Join error: {}", e))?;

            let text = extraction_result.map_err(|e| format!("Extraction failed: {}", e))?;

            println!("[create_book] Chunking text...");
            let text_for_chunks = text.clone();
            let (chunks, total_segments) = tokio::task::spawn_blocking(move || {
                let chunk_size = std::env::var("BOOK_CHUNK_SIZE").unwrap_or("500".into()).parse().unwrap_or(500);
                let chunk_overlap = std::env::var("BOOK_CHUNK_OVERLAP").unwrap_or("50".into()).parse().unwrap_or(50);
                let chunks = book_service::chunk_text(&text_for_chunks, chunk_size, chunk_overlap);
                let count = chunks.len() as i32;
                (chunks, count)
            }).await.map_err(|e| format!("Join error: {}", e))?;

            println!("[create_book] Uploading file to R2...");
            // 3. R2 Upload (Async)
            let ext = if epub { "epub" } else { "pdf" };
            let (file_url, storage_key) = match crate::services::storage_service::upload_to_r2(&file_path_for_upload, &clerk_id_clone, &slug_clone, ext).await {
                Ok((url, key)) => (url, key),
                Err(e) => return Err(format!("Upload failed: {:?}", e)),
            };
            println!("[create_book] File uploaded successfully. Key: {}", storage_key);

            println!("[create_book] Uploading cover if extracted...");
            let (cover_url, cover_key) = if let Some(cover_data) = cover_bytes {
                match crate::services::storage_service::upload_image_to_r2(&cover_data, &clerk_id_clone, &slug_clone).await {
                    Ok((url, key)) => (Some(url), Some(key)),
                    Err(e) => {
                        eprintln!("Cover upload failed (non-fatal): {:?}", e);
                        (None, None)
                    }
                }
            } else {
                (None, None)
            };

            println!("[create_book] Updating DB record...");
            // 4. Update core record
            let books_coll = db_clone.collection::<Book>("books");
            books_coll.update_one(
                doc! { "_id": book_id },
                doc! { "$set": {
                    "file_url": &file_url,
                    "storage_key": &storage_key,
                    "cover_url": &cover_url,
                    "cover_key": &cover_key,
                    "total_segments": total_segments,
                    "updated_at": DateTime::now(),
                }}
            ).await.map_err(|e| format!("DB Update failed: {}", e))?;

            println!("[create_book] Generating embeddings for segments...");
            let segment_contents: Vec<String> = chunks.iter().cloned().collect();
            let embeddings = match crate::services::gemini_service::embed_texts(segment_contents).await {
                Ok(embs) => {
                    println!("[create_book] Generated {} embeddings successfully.", embs.len());
                    Some(embs)
                }
                Err(e) => {
                    eprintln!("[create_book] Failed to generate embeddings: {:?}", e);
                    None
                }
            };

            println!("[create_book] Generating segments and chapters...");
            let segments: Vec<BookSegment> = chunks.into_iter().enumerate().map(|(i, content)| {
                let word_count = content.split_whitespace().count() as i32;
                let embedding = embeddings.as_ref().and_then(|embs| embs.get(i).cloned());
                BookSegment {
                    id: None,
                    clerk_id: clerk_id_clone.clone(),
                    book_id,
                    content,
                    segment_index: i as i32,
                    page_number: 0,
                    word_count,
                    embedding,
                    created_at: Some(DateTime::now()),
                    updated_at: Some(DateTime::now()),
                }
            }).collect();

            let chapters: Vec<BookChapter> = if epub {
                match epub_service::extract_chapters_from_epub(&file_path_for_chapters) {
                    Ok(chaps) => chaps.into_iter().enumerate().map(|(i, c)| BookChapter {
                        id: None,
                        clerk_id: clerk_id_clone.clone(),
                        book_id,
                        title: c.title,
                        html_content: c.html,
                        chapter_index: i as i32,
                        created_at: Some(DateTime::now()),
                        updated_at: Some(DateTime::now()),
                    }).collect(),
                    Err(e) => {
                        eprintln!("Chapter extraction failed: {:?}", e);
                        Vec::new()
                    }
                }
            } else {
                // PDF Chapter heuristics
                pdf_service::text_to_html_chapters(&text)
                    .into_iter().enumerate().map(|(i, c)| BookChapter {
                        id: None,
                        clerk_id: clerk_id_clone.clone(),
                        book_id,
                        title: c.title.or_else(|| Some(format!("Page {}", i+1))),
                        html_content: c.html,
                        chapter_index: i as i32,
                        created_at: Some(DateTime::now()),
                        updated_at: Some(DateTime::now()),
                    }).collect()
            };

            // Insert segments and chapters
            if !segments.is_empty() {
                let segs_coll = db_clone.collection::<BookSegment>("book_segments");
                segs_coll.insert_many(segments).await.map_err(|e| format!("Segments insert failed: {}", e))?;
            }
            if !chapters.is_empty() {
                let chaps_coll = db_clone.collection::<BookChapter>("book_chapters");
                chaps_coll.insert_many(chapters).await.map_err(|e| format!("Chapters insert failed: {}", e))?;
            }

            // Mark book as ready now that extraction and embedding succeeded
            let books_coll = db_clone.collection::<Book>("books");
            books_coll.update_one(
                doc! { "_id": book_id },
                doc! { "$set": { "processing_status": "ready" } }
            ).await.map_err(|e| format!("DB Update ready failed: {}", e))?;

            Ok(())
        }.await;

        if let Err(e) = result {
            eprintln!("[create_book] Background processing error for {}: {}", slug_clone, e);
        } else {
            println!("[create_book] Background processing COMPLETED for {}.", slug_clone);
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

    let mut tf = tempfile::NamedTempFile::new().unwrap();
    use std::io::Write;
    tf.write_all(&file_data).unwrap();
    let temp_path = tf.into_temp_path();

    let epub = is_epub(&file_data);

    // Extract title & author
    let (title, author) = if epub {
        match epub_service::extract_metadata_from_epub(&temp_path) {
            Ok(m) => (m.title, m.author),
            Err(e) => { eprintln!("EPUB metadata error: {}", e); (None, None) }
        }
    } else {
        match pdf_service::extract_metadata(&temp_path) {
            Ok(m) => (m.title, m.author),
            Err(_) => (None, None),
        }
    };

    // Extract cover preview as base64
    let cover_b64 = if epub {
        epub_service::extract_cover_from_epub(&temp_path).and_then(|(cover_bytes, _mime)| {
            use base64::{Engine as _, engine::general_purpose};
            Some(general_purpose::STANDARD.encode(&cover_bytes))
        })
    } else {
        crate::services::image_service::generate_base64_cover(&temp_path).ok()
    };

    (StatusCode::OK, Json(MetadataResponse {
        title,
        author,
        cover_b64,
    })).into_response()
}

/// GET /api/books — returns all books for the authenticated user, newest first.
pub async fn list_books(
    State(db): State<Database>,
    user: crate::middleware::auth::AuthenticatedUser,
) -> impl IntoResponse {
    let collection = db.collection::<Book>("books");

    let options = mongodb::options::FindOptions::builder()
        .sort(doc! { "created_at": -1 })
        .build();

    match collection.find(doc! { "clerk_id": &user.user_id }).with_options(options).await {
        Ok(mut cursor) => {
            let mut books: Vec<Book> = Vec::new();
            loop {
                match cursor.advance().await {
                    Ok(true) => {
                        match cursor.deserialize_current() {
                            Ok(book) => books.push(book),
                            Err(e) => eprintln!("[list_books] Deserialize error: {:?}", e),
                        }
                    }
                    Ok(false) => break,
                    Err(e) => {
                        eprintln!("[list_books] Cursor advance error: {:?}", e);
                        return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read books").into_response();
                    }
                }
            }
            (StatusCode::OK, Json(books)).into_response()
        }
        Err(e) => {
            eprintln!("[list_books] Query error: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to query books").into_response()
        }
    }
}

/// GET /api/books/public — returns all public Gutenberg books natively processed.
pub async fn list_public_books(
    State(db): State<Database>,
) -> impl IntoResponse {
    let collection = db.collection::<Book>("books");

    let options = mongodb::options::FindOptions::builder()
        .sort(doc! { "slug": 1 })
        .build();

    match collection.find(doc! { "clerk_id": "public", "processing_status": "ready" }).with_options(options).await {
        Ok(mut cursor) => {
            let mut books: Vec<Book> = Vec::new();
            while let Ok(true) = cursor.advance().await {
                if let Ok(book) = cursor.deserialize_current() {
                    books.push(book);
                }
            }
            (StatusCode::OK, Json(books)).into_response()
        }
        Err(e) => {
            eprintln!("[list_public_books] Query error: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to query public books").into_response()
        }
    }
}

/// GET /api/books/:slug — returns a single book for the authenticated user by slug.
pub async fn get_book(
    State(db): State<Database>,
    user: crate::middleware::auth::AuthenticatedUser,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    let collection = db.collection::<Book>("books");

    // Check for either the user's book OR a public Gutenberg book
    let filter = doc! { 
        "$or": [
            { "clerk_id": &user.user_id, "slug": &slug },
            { "clerk_id": "public", "slug": &slug }
        ]
    };

    match collection.find_one(filter).await {
        Ok(Some(book)) => (StatusCode::OK, Json(book)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "Book not found").into_response(),
        Err(e) => {
            eprintln!("[get_book] Query error: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to query book").into_response()
        }
    }
}

/// GET /api/books/:slug/segments — returns chunks for a book
pub async fn get_book_segments(
    State(db): State<Database>,
    user: crate::middleware::auth::AuthenticatedUser,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    let collection = db.collection::<Book>("books");

    let filter = doc! { 
        "$or": [
            { "clerk_id": &user.user_id, "slug": &slug },
            { "clerk_id": "public", "slug": &slug }
        ]
    };

    let book = match collection.find_one(filter).await {
        Ok(Some(b)) => b,
        _ => return (StatusCode::NOT_FOUND, "Book not found").into_response(),
    };

    let segments_coll = db.collection::<BookSegment>("book_segments");
    let options = mongodb::options::FindOptions::builder()
        .sort(doc! { "segment_index": 1 })
        .build();

    match segments_coll.find(doc! { "book_id": book.id.unwrap() }).with_options(options).await {
        Ok(mut cursor) => {
            let mut segments = Vec::new();
            while let Ok(true) = cursor.advance().await {
                if let Ok(seg) = cursor.deserialize_current() {
                    segments.push(seg);
                }
            }
            (StatusCode::OK, Json(segments)).into_response()
        }
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch segments").into_response()
    }
}

/// GET /api/books/:slug/chapters — returns HTML chapters for a book
pub async fn get_book_chapters(
    State(db): State<Database>,
    _user: crate::middleware::auth::AuthenticatedUser,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    let collection = db.collection::<Book>("books");

    let filter = doc! { 
        "$or": [
            { "slug": &slug }
        ]
    }; // We can be more restrictive if needed, but for viewing we just need the slug if it's there.
    
    let book = match collection.find_one(filter).await {
        Ok(Some(b)) => b,
        _ => return (StatusCode::NOT_FOUND, "Book not found").into_response(),
    };

    let chaps_coll = db.collection::<BookChapter>("book_chapters");
    let options = mongodb::options::FindOptions::builder()
        .sort(doc! { "chapter_index": 1 })
        .build();

    match chaps_coll.find(doc! { "book_id": book.id.unwrap() }).with_options(options).await {
        Ok(mut cursor) => {
            let mut chapters = Vec::new();
            while let Ok(true) = cursor.advance().await {
                if let Ok(chap) = cursor.deserialize_current() {
                    chapters.push(chap);
                }
            }
            (StatusCode::OK, Json(chapters)).into_response()
        }
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch chapters").into_response()
    }
}

#[derive(serde::Serialize)]
pub struct FileResponse {
    pub url: String,
    pub file_type: String,
}

/// GET /api/books/:slug/file — streams raw file bytes from R2 or Gutenberg
pub async fn get_book_file(
    State(db): State<Database>,
    user: crate::middleware::auth::AuthenticatedUser,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    let collection = db.collection::<Book>("books");

    let filter = doc! { 
        "$or": [
            { "clerk_id": &user.user_id, "slug": &slug },
            { "clerk_id": "public", "slug": &slug }
        ]
    };

    let book = match collection.find_one(filter).await {
        Ok(Some(b)) => b,
        _ => {
            eprintln!("[get_book_file] Book not found for slug: '{}', user: '{}'", slug, user.user_id);
            return (StatusCode::NOT_FOUND, "Book not found").into_response();
        }
    };

    eprintln!("[get_book_file] Found book '{}', storage_key='{}'", slug, book.storage_key);

    let is_epub = book.storage_key.ends_with(".epub") || book.clerk_id == "public";
    let content_type = if is_epub { "application/epub+zip" } else { "application/pdf" };

    // 1. Try R2 first (for books that have been uploaded)
    if !book.storage_key.is_empty() {
        match crate::services::storage_service::get_file_from_r2(&book.storage_key).await {
            Ok(bytes) => {
                return (
                    [
                        (axum::http::header::CONTENT_TYPE, content_type.to_string()),
                        (axum::http::header::CACHE_CONTROL, "public, max-age=3600".to_string()),
                    ],
                    bytes,
                ).into_response();
            }
            Err(e) => {
                eprintln!("[get_book_file] R2 fetch failed for key '{}': {:?}", book.storage_key, e);
            }
        }
    }

    // 2. Gutenberg fallback — fetch EPUB with User-Agent to avoid bot blocking
    if book.clerk_id == "public" {
        let id = slug.replace("gutenberg-", "");
        let urls = [
            format!("https://www.gutenberg.org/ebooks/{}.epub.images", id),
            format!("https://www.gutenberg.org/ebooks/{}.epub.noimages", id),
        ];

        let client = reqwest::Client::builder()
            .user_agent("eLib/1.0 (https://github.com/elib; contact@elib.dev)")
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        for url in &urls {
            eprintln!("[get_book_file] Trying Gutenberg: {}", url);
            if let Ok(res) = client.get(url).send().await {
                if res.status().is_success() {
                    if let Ok(bytes) = res.bytes().await {
                        // Validate: must start with PK (ZIP magic bytes) to avoid bot-blocked HTML
                        if bytes.starts_with(b"PK") {
                            return (
                                [
                                    (axum::http::header::CONTENT_TYPE, "application/epub+zip".to_string()),
                                    (axum::http::header::CACHE_CONTROL, "public, max-age=3600".to_string()),
                                ],
                                bytes.to_vec(),
                            ).into_response();
                        } else {
                            eprintln!("[get_book_file] Gutenberg returned non-EPUB data (bot block?) from {}", url);
                        }
                    }
                }
            }
        }
    }

    (StatusCode::NOT_FOUND, "File not available").into_response()
}

/// GET /api/books/gutenberg/:id
pub async fn fetch_gutenberg_book(
    State(db): State<Database>,
    _user: crate::middleware::auth::AuthenticatedUser,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let slug = format!("gutenberg-{}", id);
    let collection = db.collection::<Book>("books");

    // Check if globally cached
    if let Ok(Some(book)) = collection.find_one(doc! { "slug": &slug, "clerk_id": "public" }).await {
        // If it exists, return it immediately. The frontend polls for chapters.
        return (StatusCode::OK, Json(book)).into_response();
    }

    let book = Book {
        id: None, // Let MongoDB generate _id to avoid serde type mismatch
        clerk_id: "public".to_string(),
        title: format!("Project Gutenberg Book {}", id),
        author: Some("Fetching...".to_string()),
        persona: None,
        slug: slug.clone(),
        file_url: "".to_string(),
        storage_key: slug.clone(),
        cover_url: None,
        cover_key: None,
        file_size: 0,
        total_segments: 0,
        gutenberg_id: Some(id.parse::<i64>().unwrap_or(0)),
        processing_status: Some("pending".to_string()),
        created_at: DateTime::now(),
        updated_at: DateTime::now(),
    };

    let books_coll = db.collection::<Book>("books");
    
    // Attempt insertion — handle race conditions where another thread beat us to it
    let book_id = match books_coll.insert_one(book.clone()).await {
        Ok(r) => r.inserted_id.as_object_id().expect("MongoDB generates ObjectId"),
        Err(e) => {
            if e.to_string().contains("E11000") {
                if let Ok(Some(existing_book)) = collection.find_one(doc! { "slug": &slug, "clerk_id": "public" }).await {
                    return (StatusCode::OK, Json(existing_book)).into_response();
                }
            }
            eprintln!("[gutenberg] Failed to insert placeholder: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to initialize book").into_response();
        }
    };

    let db_clone = db.clone();
    let id_clone = id.clone();
    let slug_clone = slug.clone();

    // 2. Background Ingestion — download EPUB, extract text/chapters, chunk for RAG
    tokio::spawn(async move {
        let result: Result<(), String> = async {
            // Build a client with User-Agent to avoid Gutenberg bot blocking
            let client = reqwest::Client::builder()
                .user_agent("eLib/1.0 (https://github.com/elib; contact@elib.dev)")
                .redirect(reqwest::redirect::Policy::limited(5))
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .map_err(|e| format!("Client build error: {}", e))?;

            // 1. Download EPUB bytes
            let epub_urls = [
                format!("https://www.gutenberg.org/ebooks/{}.epub.images", id_clone),
                format!("https://www.gutenberg.org/ebooks/{}.epub.noimages", id_clone),
            ];

            let mut epub_bytes: Option<Vec<u8>> = None;
            for url in &epub_urls {
                eprintln!("[gutenberg bg] Trying: {}", url);
                if let Ok(res) = client.get(url).send().await {
                    if res.status().is_success() {
                        if let Ok(bytes) = res.bytes().await {
                            if bytes.starts_with(b"PK") {
                                eprintln!("[gutenberg bg] Downloaded {} bytes from {}", bytes.len(), url);
                                epub_bytes = Some(bytes.to_vec());
                                break;
                            } else {
                                eprintln!("[gutenberg bg] Non-EPUB data from {} (bot block?)", url);
                            }
                        }
                    }
                }
            }

            let epub_bytes = epub_bytes.ok_or("All Gutenberg download URLs failed".to_string())?;

            // 2. Write to temp file for processing
            let temp_file = tempfile::NamedTempFile::new().map_err(|e| e.to_string())?;
            let temp_path = temp_file.path().to_path_buf();
            std::fs::write(&temp_path, &epub_bytes).map_err(|e| format!("Write temp: {}", e))?;

            // 3. Extract metadata, text, chapters (blocking CPU work)
            let temp_path_clone = temp_path.clone();
            let (text_result, chapters_result, metadata, _cover_bytes) = tokio::task::spawn_blocking(move || {
                let text = epub_service::extract_text_from_epub(&temp_path_clone);
                let chapters = epub_service::extract_chapters_from_epub(&temp_path_clone);
                let meta = epub_service::extract_metadata_from_epub(&temp_path_clone).ok();
                let cover = epub_service::extract_cover_from_epub(&temp_path_clone).map(|(b, _)| b);
                (text, chapters, meta, cover)
            }).await.map_err(|e| format!("Join error: {}", e))?;

            let text = text_result.map_err(|e| format!("Text extraction failed: {}", e))?;
            let chapters_vec = chapters_result.unwrap_or_default();

            // 4. Chunk text for RAG (500-word segments)
            let text_for_chunks = text.clone();
            let (chunks, total_segments) = tokio::task::spawn_blocking(move || {
                let chunk_size = std::env::var("BOOK_CHUNK_SIZE").unwrap_or("500".into()).parse().unwrap_or(500);
                let chunk_overlap = std::env::var("BOOK_CHUNK_OVERLAP").unwrap_or("50".into()).parse().unwrap_or(50);
                let chunks = book_service::chunk_text(&text_for_chunks, chunk_size, chunk_overlap);
                let count = chunks.len() as i32;
                (chunks, count)
            }).await.map_err(|e| format!("Join error: {}", e))?;

            // 5. Upload EPUB to R2
            let (file_url, storage_key) = match crate::services::storage_service::upload_to_r2(&temp_path, "public", &slug_clone, "epub").await {
                Ok((url, key)) => (url, key),
                Err(e) => return Err(format!("R2 upload failed: {:?}", e)),
            };

            // 6. Cover Image
            let cover_url = Some(format!("https://www.gutenberg.org/cache/epub/{}/pg{}.cover.medium.jpg", id_clone, id_clone));
            let cover_key: Option<String> = None;

            // 7. Update book record with real metadata
            let title = metadata.as_ref().and_then(|m| m.title.clone())
                .unwrap_or(format!("Gutenberg Book {}", id_clone));
            let author = metadata.as_ref().and_then(|m| m.author.clone())
                .unwrap_or("Unknown".to_string());

            let books_coll = db_clone.collection::<Book>("books");
            books_coll.update_one(
                doc! { "_id": book_id },
                doc! { "$set": {
                    "title": &title,
                    "author": &author,
                    "file_url": &file_url,
                    "storage_key": &storage_key,
                    "cover_url": &cover_url,
                    "cover_key": &cover_key,
                    "total_segments": total_segments,
                    "file_size": epub_bytes.len() as i64,
                    "updated_at": DateTime::now(),
                }}
            ).await.map_err(|e| format!("DB update failed: {}", e))?;

            // 8. Insert RAG segments
            eprintln!("[gutenberg bg] Generating embeddings for Gutenberg segments...");
            let segment_contents: Vec<String> = chunks.iter().cloned().collect();
            let embeddings = match crate::services::gemini_service::embed_texts(segment_contents).await {
                Ok(embs) => {
                    eprintln!("[gutenberg bg] Generated {} embeddings successfully.", embs.len());
                    Some(embs)
                }
                Err(e) => {
                    eprintln!("[gutenberg bg] Failed to generate embeddings: {:?}", e);
                    None
                }
            };

            let segments: Vec<BookSegment> = chunks.into_iter().enumerate().map(|(i, content)| {
                let word_count = content.split_whitespace().count() as i32;
                let embedding = embeddings.as_ref().and_then(|embs| embs.get(i).cloned());
                BookSegment {
                    id: None,
                    clerk_id: "public".into(),
                    book_id,
                    content,
                    segment_index: i as i32,
                    page_number: 0,
                    word_count,
                    embedding,
                    created_at: Some(DateTime::now()),
                    updated_at: Some(DateTime::now()),
                }
            }).collect();

            if !segments.is_empty() {
                let segs_coll = db_clone.collection::<BookSegment>("book_segments");
                segs_coll.insert_many(segments).await.map_err(|e| format!("Segments insert: {}", e))?;
            }

            // 9. Insert HTML chapters
            let chapters: Vec<BookChapter> = chapters_vec.into_iter().enumerate().map(|(i, c)| {
                BookChapter {
                    id: None,
                    clerk_id: "public".into(),
                    book_id,
                    title: c.title,
                    html_content: c.html,
                    chapter_index: i as i32,
                    created_at: Some(DateTime::now()),
                    updated_at: Some(DateTime::now()),
                }
            }).collect();

            if !chapters.is_empty() {
                let num_chapters = chapters.len();
                let chaps_coll = db_clone.collection::<BookChapter>("book_chapters");
                chaps_coll.insert_many(chapters).await.map_err(|e| format!("Chapters insert: {}", e))?;
                eprintln!("[gutenberg bg] ✅ Finished processing {}: {} segments, {} chapters",
                    slug_clone, total_segments, num_chapters);
            } else {
                eprintln!("[gutenberg bg] ✅ Finished processing {}: {} segments, 0 chapters",
                    slug_clone, total_segments);
            }

            // Mark Gutenberg cache book as ready
            let books_coll = db_clone.collection::<Book>("books");
            books_coll.update_one(
                doc! { "_id": book_id },
                doc! { "$set": { "processing_status": "ready" } }
            ).await.map_err(|e| format!("DB Update ready failed: {}", e))?;
            Ok(())
        }.await;

        if let Err(e) = result {
            eprintln!("[gutenberg] Background processing error for {}: {}", slug_clone, e);
        }
    });

    (StatusCode::OK, Json(book)).into_response()
}
#[derive(Deserialize)]
pub struct ClaimRequest {
    pub book_id: String,
}

pub async fn claim_book(
    State(db): State<Database>,
    user: crate::middleware::auth::AuthenticatedUser,
    Json(payload): Json<ClaimRequest>,
) -> impl IntoResponse {
    let book_oid = match ObjectId::parse_str(&payload.book_id) {
        Ok(oid) => oid,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid book ID").into_response(),
    };

    let books_coll = db.collection::<Book>("books");
    let segments_coll = db.collection::<BookSegment>("book_segments");

    // Check if the book is actually anonymous
    let filter = doc! { "_id": book_oid, "clerk_id": "anonymous" };
    let update = doc! { "$set": { "clerk_id": &user.user_id } };

    match books_coll.update_one(filter, update).await {
        Ok(res) if res.matched_count > 0 => {
            // Also update segments
            let _ = segments_coll.update_many(
                doc! { "book_id": book_oid, "clerk_id": "anonymous" },
                doc! { "$set": { "clerk_id": &user.user_id } }
            ).await;
            (StatusCode::OK, "Book claimed successfully").into_response()
        }
        Ok(_) => (StatusCode::NOT_FOUND, "Book already claimed or not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {}", e)).into_response(),
    }
}
