use std::collections::HashMap;

use pb::scuffle::video::v1::types::{AudioConfig, RecordingConfig, TranscodingConfig, VideoConfig};

use super::{Protobuf, RoomStatus, TraitProtobuf, TraitProtobufVec, Ulid};

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct Room {
    pub id: Ulid,
    pub organization_id: Ulid,

    pub transcoding_config_id: Option<Ulid>,
    pub recording_config_id: Option<Ulid>,

    pub private: bool,

    pub stream_key: String,
    pub updated_at: chrono::DateTime<chrono::Utc>,

    pub last_live_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_disconnected_at: Option<chrono::DateTime<chrono::Utc>>,

    pub status: RoomStatus,

    pub video_input: Option<Protobuf<VideoConfig>>,
    pub audio_input: Option<Protobuf<AudioConfig>>,

    pub active_ingest_connection_id: Option<Ulid>,
    pub active_recording_config: Option<Protobuf<RecordingConfig>>,
    pub active_transcoding_config: Option<Protobuf<TranscodingConfig>>,
    pub active_recording_id: Option<Ulid>,

    pub ingest_bitrate: Option<i32>,

    pub video_output: Option<Vec<Protobuf<VideoConfig>>>,
    pub audio_output: Option<Vec<Protobuf<AudioConfig>>>,

    pub tags: sqlx::types::Json<HashMap<String, String>>,
}

impl Room {
    pub fn into_proto(self) -> pb::scuffle::video::v1::types::Room {
        pb::scuffle::video::v1::types::Room {
            id: Some(self.id.0.into()),
            transcoding_config_id: self.transcoding_config_id.map(|id| id.0.into()),
            recording_config_id: self.recording_config_id.map(|id| id.0.into()),
            private: self.private,
            stream_key: self.stream_key,
            created_at: self.id.0.timestamp_ms() as i64,
            updated_at: self.updated_at.timestamp_millis(),
            last_live_at: self.last_live_at.map(|t| t.timestamp_millis()),
            last_disconnected_at: self.last_disconnected_at.map(|t| t.timestamp_millis()),
            status: self.status.into(),
            audio_input: self.audio_input.map(|a| a.into_inner()),
            video_input: self.video_input.map(|v| v.into_inner()),
            audio_output: self.audio_output.map(|a| a.into_vec()).unwrap_or_default(),
            video_output: self.video_output.map(|v| v.into_vec()).unwrap_or_default(),
            active_recording_id: self.active_recording_id.map(|r| r.0.into()),
            active_connection_id: self.active_ingest_connection_id.map(|c| c.0.into()),
            tags: Some(self.tags.0.into()),
        }
    }
}
