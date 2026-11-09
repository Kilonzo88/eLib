use mongodb::bson::{DateTime, oid::ObjectId};
use serde::{Deserialize, Serialize};

/// Equivalent to book-segment.model.ts — maps to the `book_segments` collection.
/// Critical for RAG: Atlas Automated Embeddings indexes `content` with Voyage `voyage-4`,
/// so retrieval searches the text directly instead of a stored embedding field.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BookSegment {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,

    pub clerk_id: String,  // isolates segments by user for security
    pub book_id: ObjectId, // reference to parent Book

    pub content: String, // embedded by Atlas Automated Embeddings (voyage-4)

    pub segment_index: i32, // indexed — fast jumping by position
    pub page_number: i32,   // indexed — fast jumping to page
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_start: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_end: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chapter_index: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chapter_title: Option<String>,
    pub word_count: i32, // used to ensure chunking constraints

    // Timestamps
    pub created_at: Option<DateTime>,
    pub updated_at: Option<DateTime>,
}
