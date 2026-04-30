pub mod db;
pub mod models;
pub mod routes;
pub mod services;
pub mod middleware;

use axum::{routing::get, Router};
use mongodb::bson::doc;
use mongodb::IndexModel;
use mongodb::options::IndexOptions;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    // Load environment variables from .env
    dotenvy::dotenv().ok();

    println!("Initializing eLib backend...");

    // Establish MongoDB connection (pooled natively by the driver)
    let database = db::connect().await;

    // ── Indexes ──────────────────────────────────────────────────────────────
    // books: unique index on slug
    let books = database.collection::<mongodb::bson::Document>("books");
    books.create_index(
        IndexModel::builder()
            .keys(doc! { "slug": 1 })
            .options(IndexOptions::builder().unique(true).build())
            .build(),
    )
    .await
    .expect("Failed to create books.slug index");

    // book_segments: compound text index & compound lookups to scope by book_id!
    let segments = database.collection::<mongodb::bson::Document>("book_segments");
    segments.create_index(
        IndexModel::builder()
            .keys(doc! { "book_id": 1, "content": "text" })
            .build(),
    )
    .await
    .expect("Failed to create book_segments text index");

    segments.create_index(
        IndexModel::builder()
            .keys(doc! { "book_id": 1, "segment_index": 1 })
            .options(IndexOptions::builder().unique(true).build())
            .build(),
    )
    .await
    .expect("Failed to create book_segments segment_index unique index");

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

    // ── Router ────────────────────────────────────────────────────────────────
    let app = Router::new()
        .nest("/api", routes::create_router(database.clone()))
        .route("/", get(|| async { "eLib Backend Running!" }));

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
