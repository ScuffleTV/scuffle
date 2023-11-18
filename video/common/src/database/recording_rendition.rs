use common::database::{Protobuf, Ulid};

use super::Rendition;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RecordingRendition {
	pub recording_id: Ulid,
	pub rendition: Rendition,
	pub config: Protobuf<Vec<u8>>,
}
