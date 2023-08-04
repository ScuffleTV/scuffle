use uuid::Uuid;

use crate::rendition::Rendition;


#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RecordingRendition {
    pub recording_id: Uuid,
    pub rendition: Rendition,

    pub organization_id: Uuid,
    pub segment_ids: Vec<Uuid>,
    pub segment_durations: Vec<i32>,
    pub timescale: i32,
    pub total_size: i64,

    #[sqlx(default)]
    pub public_url: Option<String>,
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
