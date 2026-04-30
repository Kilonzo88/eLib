use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};

/// Equivalent to book.model.ts — maps to the `books` collection.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Book {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,

    pub clerk_id: String,     // links to the authenticated Clerk user
    pub title: String,
    pub author: String,
    pub persona: Option<String>,

    pub slug: String,         // unique, lowercase URL slug

    pub file_url: Option<String>,
    pub file_blob_key: Option<String>,
    pub cover_url: Option<String>,
    pub cover_blob_key: Option<String>,

    pub file_size: Option<i64>,
    pub total_segments: Option<i32>,

    // Timestamps — set manually since MongoDB driver doesn't auto-manage these
    pub created_at: Option<DateTime>,
    pub updated_at: Option<DateTime>,
}

/// Payload used when creating a new book (no ID or timestamps yet).
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateBookPayload {
    pub clerk_id: String,
    pub title: String,
    pub author: String,
    pub persona: Option<String>,
    pub slug: String,
    pub file_url: Option<String>,
    pub file_blob_key: Option<String>,
    pub cover_url: Option<String>,
    pub cover_blob_key: Option<String>,
    pub file_size: Option<i64>,
}
