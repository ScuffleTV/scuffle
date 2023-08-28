use super::{Protobuf, Rendition, Ulid};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RecordingRendition {
    pub recording_id: Ulid,
    pub rendition: Rendition,
    pub config: Protobuf<Vec<u8>>,
}
