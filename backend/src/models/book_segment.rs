use mongodb::bson::{oid::ObjectId};
use serde::{Deserialize, Serialize};

/// Equivalent to book-segment.model.ts — maps to the `book_segments` collection.
/// Critical for RAG: content is text-indexed so VAPI can find relevant context instantly.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BookSegment {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,

    pub clerk_id: String,       // isolates segments by user for security
    pub book_id: ObjectId,      // reference to parent Book

    pub content: String,        // text-indexed for full-text search

    pub segment_index: i32,     // indexed — fast jumping by position
    pub page_number: i32,       // indexed — fast jumping to page
    pub word_count: i32,        // used to ensure chunking constraints
    
    // Timestamps
    pub created_at: Option<DateTime>,
    pub updated_at: Option<DateTime>,
}
