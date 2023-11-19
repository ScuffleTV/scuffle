use common::database::{Protobuf, Ulid};

use super::{DatabaseTable, Rendition};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RecordingRendition {
	pub recording_id: Ulid,
	pub rendition: Rendition,
	pub config: Protobuf<Vec<u8>>,
}

impl DatabaseTable for RecordingRendition {
	const FRIENDLY_NAME: &'static str = "recording rendition";
	const NAME: &'static str = "recording_renditions";
}
