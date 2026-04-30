use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};

/// Equivalent to voice-session.model.ts — maps to the `voice_sessions` collection.
/// Tracks AI interactions per user per book, used for billing and quota enforcement.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VoiceSession {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,

    pub clerk_id: String,       // compound-indexed with billing_period_start
    pub book_id: ObjectId,

    pub started_at: DateTime,
    pub ended_at: Option<DateTime>,
    pub duration_seconds: Option<i32>,

    pub billing_period_start: DateTime, // indexed with clerk_id for quota tracking
}
