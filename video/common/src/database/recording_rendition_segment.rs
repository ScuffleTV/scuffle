use common::database::Ulid;

use super::Rendition;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RecordingRenditionSegment {
    pub recording_id: Ulid,
    pub rendition: Rendition,
    pub idx: i32,
    pub id: Ulid,
    pub start_time: f32,
    pub end_time: f32,
    pub size_bytes: i32,
}
