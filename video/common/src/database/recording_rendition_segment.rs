use common::database::Ulid;

use super::{DatabaseTable, Rendition};

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

impl DatabaseTable for RecordingRenditionSegment {
	const FRIENDLY_NAME: &'static str = "recording rendition segment";
	const NAME: &'static str = "recording_rendition_segments";
}
