use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};

/// Represents a structural segment of a book (e.g., a chapter) containing fully formatted HTML.
/// Used exclusively for native Frontend rendering to preserve headings and layout.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BookChapter {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,

    pub clerk_id: String,
    pub book_id: ObjectId,

    pub title: Option<String>,  // e.g., "Chapter 1", or None if unnamed
    pub html_content: String,   // Raw XHTML / HTML extracted from the EPUB

    pub chapter_index: i32,     // For sequential ordering

    pub created_at: Option<DateTime>,
    pub updated_at: Option<DateTime>,
}
