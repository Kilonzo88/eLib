pub mod db;
pub mod models;
pub mod routes;
pub mod services;
pub mod middleware;

use axum::{routing::get, Router, extract::DefaultBodyLimit};
use mongodb::bson::doc;
use mongodb::IndexModel;
use mongodb::options::IndexOptions;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() {
    // Load environment variables from .env
    dotenvy::dotenv().ok();

    println!("Initializing eLib backend...");

    // Establish MongoDB connection (pooled natively by the driver)
    let database = db::connect().await;

    // Verify Cloudflare R2 Connectivity
    match services::storage_service::validate_config().await {
        Ok(_) => println!("Successfully connected to Cloudflare R2!"),
        Err(e) => {
            eprintln!("Warning: Failed to connect to Cloudflare R2: {:?}", e);
            eprintln!("Check your R2_ACCOUNT_ID, ACCESS_KEY, and SECRET_KEY in .env");
            // We don't panic here to allow local dev without R2 if needed, 
            // but we could .expect() if it's strictly required.
        }
    }

    // ── Indexes ──────────────────────────────────────────────────────────────
    // books: unique compound index on clerk_id and slug to prevent user A and user B from having a book name collission
    let books = database.collection::<mongodb::bson::Document>("books");
    books.create_index(
        IndexModel::builder()
            .keys(doc! { "clerk_id": 1, "slug": 1 })
            .options(IndexOptions::builder().unique(true).build())
            .build(),
    )
    .await
    .expect("Failed to create books compound index on clerk_id and slug");

    // book_segments: compound text index & compound lookups to scope by book_id!. Builds a map of every scanned word in the book.
    let segments = database.collection::<mongodb::bson::Document>("book_segments");
    segments.create_index(
        IndexModel::builder()
            .keys(doc! { "content": "text" })
            .build(),
    )
    .await
    .expect("Failed to create book_segments text index");

    // book_segments: compound unique index on book_id and segment_index to prevent duplicate segments. Also used to keep segments in order.
    segments.create_index(
        IndexModel::builder()
            .keys(doc! { "book_id": 1, "segment_index": 1 })
            .options(IndexOptions::builder().unique(true).build())
            .build(),
    )
    .await
    .expect("Failed to create book_segments segment_index unique index");

    // book_segments: compound index on book_id and page_number to scope by book_id and page_number
    segments.create_index(
        IndexModel::builder()
            .keys(doc! { "book_id": 1, "page_number": 1 })
            .build(),
    )
    .await
    .expect("Failed to create book_segments page_number index");

    // voice_sessions: compound index for billing quota lookups
    let sessions = database.collection::<mongodb::bson::Document>("voice_sessions");
    sessions.create_index(
        IndexModel::builder()
            .keys(doc! { "clerk_id": 1, "billing_period_start": 1 })
            .build(),
    )
    .await
    .expect("Failed to create voice_sessions billing index");

    println!("All indexes ensured.");

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // ── Router ────────────────────────────────────────────────────────────────
    let app = Router::new()
        .nest("/api", routes::create_router(database.clone()))
        .layer(DefaultBodyLimit::max(50 * 1024 * 1024)) // 50MB
        .layer(cors)
        .route("/", get(|| async { "eLib Backend Running!" }));

    let addr = SocketAddr::from(([0, 0, 0, 0], 8081));
    println!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
