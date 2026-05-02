use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};

/// Equivalent to book.model.ts — maps to the `books` collection.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Book {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,

    pub clerk_id: String, // links to the authenticated Clerk user
    pub title: String,
    pub author: Option<String>,
    pub persona: Option<String>,

    pub slug: String, // unique, lowercase URL slug

    pub file_url: String,
    pub storage_key: String,
    pub cover_url: Option<String>,
    pub cover_key: Option<String>,

    pub file_size: i64,
    pub total_segments: i32,

    // Timestamps — set manually since MongoDB driver doesn't auto-manage these
    pub created_at: DateTime,
    pub updated_at: DateTime,
}
