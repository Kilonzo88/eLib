use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize, Serializer};

/// Serialise an `Option<ObjectId>` as a plain hex string (e.g. "6634abc123…")
/// instead of the extended-JSON object `{"$oid":"…"}` that the default
/// BSON serialiser emits.  This makes the value safe to use as a React key.
fn serialize_object_id_as_hex_string<S>(
    id: &Option<ObjectId>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match id {
        Some(oid) => serializer.serialize_str(&oid.to_hex()),
        None => serializer.serialize_none(),
    }
}

/// Equivalent to book.model.ts — maps to the `books` collection.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Book {
    #[serde(
        rename = "_id",
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_object_id_as_hex_string"
    )]
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

    pub gutenberg_id: Option<i64>,
    pub processing_status: Option<String>,

    // Timestamps — set manually since MongoDB driver doesn't auto-manage these
    pub created_at: DateTime,
    pub updated_at: DateTime,
}
