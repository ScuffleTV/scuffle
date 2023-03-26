use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct Model {
    /// The unique identifier for the stream variant.
    pub id: Uuid,
    /// The unique identifier for the stream.
    pub stream_id: Uuid,
    /// The name of the stream variant.
    pub name: String,
    /// The width of the stream variant. (if null then the stream variant is not a video stream)
    pub video_width: Option<i64>,
    /// The height of the stream variant. (if null then the stream variant is not a video stream)
    pub video_height: Option<i64>,
    /// The framerate of the stream variant. (if null then the stream variant is not a video stream)
    pub video_framerate: Option<i64>,
    /// The bandwidth in bits/s of the stream variant.
    pub video_bitrate: Option<i64>,
    /// Video codec of the stream variant.
    pub video_codec: Option<String>,
    /// The audio sample rate of the stream variant.
    pub audio_sample_rate: Option<i64>,
    /// The number of audio channels of the stream variant.
    pub audio_channels: Option<i64>,
    /// The bandwidth in bits/s of the stream variant.
    pub audio_bitrate: Option<i64>,
    // Audio Codec of the stream variant.
    pub audio_codec: Option<String>,
    /// Metadata
    pub metadata: Value,
    /// The time the stream variant was created.
    pub created_at: DateTime<Utc>,
}
