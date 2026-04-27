pub mod models;
pub mod routes;
pub mod services;
pub mod middleware;

use axum::{routing::get, Router};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    println!("Initializing eLib backend...");

    let app = Router::new().route("/", get(|| async { "eLib Backend Running!" }));

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("Listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
