use uuid::Uuid;

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct Recording {
    pub id: Uuid,
    pub organization_id: Uuid,

    pub room_id: Option<Uuid>,
    pub recording_config_id: Option<Uuid>,

    pub public: bool,
    pub deleted: bool,
    pub allow_dvr: bool,

    pub updated_at: chrono::DateTime<chrono::Utc>,
}

// impl Recording {
//     pub fn into_proto(self, info: RecordingInfo) -> pb::scuffle::video::v1::types::Recording {
//         pb::scuffle::video::v1::types::Recording {
//             id: Some(self.id.into()),
//             room_id: self.room_id.map(|id| id.into()),
//             recording_config_id: self.recording_config_id.map(|id| id.into()),
//             video_renditions: self
//                 .video_renditions
//                 .into_iter()
//                 .map(|r| PbRenditionVideo::from(r).into())
//                 .collect(),
//             audio_renditions: self
//                 .audio_renditions
//                 .into_iter()
//                 .map(|r| PbRenditionAudio::from(r).into())
//                 .collect(),
//             created_at: Ulid::from(self.id).timestamp_ms() as i64,
//             updated_at: self.updated_at.timestamp_millis(),
//             ended_at: self.ended_at.map(|t| t.timestamp_millis()),
//             byte_size: info.total_size,
//             duration: info.recording_duration as f32,
//         }
//     }
// }
