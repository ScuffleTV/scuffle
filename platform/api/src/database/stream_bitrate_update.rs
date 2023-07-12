use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct Model {
    pub stream_id: Uuid,
    pub video_bitrate: i64,
    pub audio_bitrate: i64,
    pub metadata_bitrate: i64,
    pub created_at: DateTime<Utc>,
}
