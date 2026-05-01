use axum::{Router, routing::post};
use mongodb::Database;

pub mod book;

pub fn create_router(db: Database) -> Router {
    Router::new()
        .route("/books", post(book::create_book))
        .with_state(db)
}
