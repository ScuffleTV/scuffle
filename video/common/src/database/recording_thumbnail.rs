use common::database::Ulid;

use super::DatabaseTable;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RecordingThumbnail {
	pub recording_id: Ulid,
	pub idx: i32,
	pub id: Ulid,
	pub start_time: f32,
	pub size_bytes: i64,
}

impl DatabaseTable for RecordingThumbnail {
	const FRIENDLY_NAME: &'static str = "recording thumbnail";
	const NAME: &'static str = "recording_thumbnails";
}
