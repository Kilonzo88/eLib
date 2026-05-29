use axum::{routing::get, routing::post, Router};
use mongodb::Database;

pub mod book;

pub fn create_router(db: Database) -> Router {
    Router::new()
        .route("/books", post(book::create_book).get(book::list_books))
        .route("/books/extract-metadata", post(book::extract_metadata))
        .route("/books/public", get(book::list_public_books))
        .route("/books/:slug", get(book::get_book))
        .route("/books/:slug/segments", get(book::get_book_segments))
        .route("/books/:slug/chapters", get(book::get_book_chapters))
        .route("/books/:slug/file", get(book::get_book_file))
        .route("/books/claim", post(book::claim_book))
        .route("/books/gutenberg/:id", get(book::fetch_gutenberg_book))
        .with_state(db)
}
