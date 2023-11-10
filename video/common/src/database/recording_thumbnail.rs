use common::database::Ulid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RecordingThumbnail {
    pub recording_id: Ulid,
    pub idx: i32,
    pub id: Ulid,
    pub start_time: f32,
    pub size_bytes: i64,
}
