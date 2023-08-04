use std::collections::HashMap;

use pb::scuffle::video::v1::types::{AudioConfig, RecordingConfig, TranscodingConfig, VideoConfig};
use ulid::Ulid;
use uuid::Uuid;

use crate::{
    adapter::{Adapter, TraitAdapter, TraitAdapterVec},
    room_status::RoomStatus,
};

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct Room {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub transcoding_config_id: Option<Uuid>,
    pub recording_config_id: Option<Uuid>,
    pub private: bool,
    pub stream_key: String,
    pub updated_at: chrono::DateTime<chrono::Utc>,

    pub last_live_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_disconnected_at: Option<chrono::DateTime<chrono::Utc>>,

    pub status: RoomStatus,

    pub video_input: Option<Adapter<VideoConfig>>,
    pub audio_input: Option<Adapter<AudioConfig>>,

    pub video_output: Option<Vec<Adapter<VideoConfig>>>,
    pub audio_output: Option<Vec<Adapter<AudioConfig>>>,

    pub active_recording_config: Option<Adapter<RecordingConfig>>,
    pub active_transcoding_config: Option<Adapter<TranscodingConfig>>,
    pub active_ingest_connection_id: Option<Uuid>,
    pub active_recording_id: Option<Uuid>,
    pub tags: Vec<String>,
}

impl Room {
    pub fn into_proto(self) -> pb::scuffle::video::v1::types::Room {
        pb::scuffle::video::v1::types::Room {
            id: Some(self.id.into()),
            transcoding_config_id: self.transcoding_config_id.map(|id| id.into()),
            recording_config_id: self.recording_config_id.map(|id| id.into()),
            private: self.private,
            stream_key: self.stream_key,
            created_at: Ulid::from(self.id).timestamp_ms() as i64,
            updated_at: self.updated_at.timestamp_millis(),
            last_live_at: self.last_live_at.map(|t| t.timestamp_millis()),
            last_disconnected_at: self.last_disconnected_at.map(|t| t.timestamp_millis()),
            status: self.status.into(),
            audio_input: self.audio_input.map(|a| a.into_inner()),
            video_input: self.video_input.map(|v| v.into_inner()),
            audio_output: self.audio_output.map(|a| a.into_vec()).unwrap_or_default(),
            video_output: self.video_output.map(|v| v.into_vec()).unwrap_or_default(),
            active_recording_id: self.active_recording_id.map(|r| r.into()),
            active_connection_id: self.active_ingest_connection_id.map(|c| c.into()),
            tags: self.tags.iter().map(|s| {
                let splits = s.splitn(2, ':').collect::<Vec<_>>();

                if splits.len() == 2 {
                    (splits[0].to_string(), splits[1].to_string())
                } else {
                    (splits[0].to_string(), "".to_string())
                }
            }).collect::<HashMap<_, _>>(),
        }
    }
}
