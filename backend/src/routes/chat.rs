use axum::{
    extract::{State, Path},
    Json,
    response::IntoResponse,
    http::StatusCode
};
use mongodb::Database;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use crate::models::book::Book;
use crate::models::book_segment::BookSegment;
use crate::services::gemini_service;

#[derive(Deserialize)]
pub struct ChatRequest {
    pub query: String,
    pub selected_text: Option<String>,
    pub history: Vec<serde_json::Value>, // expect format: [{"role": "user"|"model", "text": "..."}]
}

#[derive(Serialize)]
pub struct ChatResponse {
    pub response: String,
}

pub async fn chat_with_book(
    State(db): State<Database>,
    user: crate::middleware::auth::AuthenticatedUser,
    Path(slug): Path<String>,
    Json(payload): Json<ChatRequest>,
) -> impl IntoResponse {
    println!("[chat_with_book] Slug: {}, Query: {}", slug, payload.query);

    // 1. Fetch the book to verify ownership/access
    let books_coll = db.collection::<Book>("books");
    let filter = doc! {
        "$or": [
            { "clerk_id": &user.user_id, "slug": &slug },
            { "clerk_id": "public", "slug": &slug }
        ]
    };

    let book = match books_coll.find_one(filter).await {
        Ok(Some(b)) => b,
        Ok(None) => return (StatusCode::NOT_FOUND, "Book not found").into_response(),
        Err(e) => {
            eprintln!("[chat_with_book] Fetch book failed: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    };

    let book_id = match book.id {
        Some(id) => id,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error: missing book ID").into_response(),
    };

    // 2. Generate embedding for user's query
    let query_embedding = match gemini_service::embed_text(&payload.query).await {
        Ok(emb) => emb,
        Err(e) => {
            eprintln!("[chat_with_book] Failed to embed query: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to generate query embedding").into_response();
        }
    };

    // 3. Query MongoDB for relevant book segments using Atlas Vector Search
    let clerk_filter = if book.clerk_id == "public" {
        "public".to_string()
    } else {
        user.user_id.clone()
    };

    let vector_search_stage = doc! {
        "$vectorSearch": {
            "index": "vector_index", // Name of the Index configured in MongoDB Atlas
            "path": "embedding",
            "queryVector": query_embedding,
            "numCandidates": 100,
            "limit": 4, // retrieve top 4 segments
            "filter": doc! {
                "book_id": book_id,
                "clerk_id": clerk_filter
            }
        }
    };

    let segments_coll = db.collection::<BookSegment>("book_segments");
    let mut cursor = match segments_coll.aggregate(vec![vector_search_stage]).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[chat_with_book] MongoDB Atlas Vector Search failed: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("Vector search query failed: {}", e)).into_response();
        }
    };

    let mut context_chunks = Vec::new();
    while let Ok(true) = cursor.advance().await {
        if let Ok(doc) = cursor.deserialize_current() {
            if let Ok(segment) = mongodb::bson::from_document::<BookSegment>(doc) {
                context_chunks.push(segment.content);
            }
        }
    }

    // 4. Formulate System Prompt and context.
    let context_string = context_chunks.join("\n\n---\n\n");
    let system_instruction = format!(
        "You are an expert AI companion for eLib, a digital reading platform. \
        The user is reading the book \"{}\" by {}.\n\n\
        Here is the relevant context retrieved from the book:\n\n{}\n\n\
        Use this context to answer the user's question accurately. \
        If the context does not contain enough information to answer, use your pre-trained knowledge about the book, but make sure to declare that it was not found in the immediate chunks.",
        book.title,
        book.author.unwrap_or_else(|| "Unknown".to_string()),
        context_string
    );

    // Format query text to inject active selections, if any.
    let mut formatted_query = payload.query.clone();
    if let Some(ref selection) = payload.selected_text {
        formatted_query = format!(
            "CONTEXT DETAIL: The user has highlighted the following text in the book: \"{}\"\n\n\
            QUESTION ABOUT THE HIGHLIGHTED TEXT:\n{}",
            selection,
            payload.query
        );
    }

    // 5. Generate Response via Gemini
    match gemini_service::generate_chat(&system_instruction, payload.history, &formatted_query).await {
        Ok(text) => (StatusCode::OK, Json(ChatResponse { response: text })).into_response(),
        Err(e) => {
            eprintln!("[chat_with_book] Gemini chat generation failed: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to generate response").into_response()
        }
    }
}
