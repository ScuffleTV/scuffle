use std::collections::HashMap;

use common::database::Ulid;

use super::{DatabaseTable, Rendition, Visibility};

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct Recording {
	/// The organization this recording belongs to (primary key)
	pub organization_id: Ulid,
	/// A unique id for the recording (primary key)
	pub id: Ulid,

	/// The room this recording belongs to
	pub room_id: Option<Ulid>,

	/// The recording config this recording uses
	pub recording_config_id: Option<Ulid>,

	/// The S3 bucket this recording uses
	pub s3_bucket_id: Ulid,

	/// If the recording is public
	pub visibility: Visibility,

	/// If the recording allows for DVR playback while being recorded
	pub allow_dvr: bool,

	/// The date and time the recording was deleted.
	pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,

	/// The date and time the recording was last updated
	pub updated_at: chrono::DateTime<chrono::Utc>,

	/// The date and time the recording ended
	pub ended_at: Option<chrono::DateTime<chrono::Utc>>,

	#[sqlx(json)]
	/// The tags associated with the recording
	pub tags: HashMap<String, String>,
}

impl DatabaseTable for Recording {
	const FRIENDLY_NAME: &'static str = "recording";
	const NAME: &'static str = "recordings";
}

impl Recording {
	pub fn into_proto(
		self,
		renditions: Vec<Rendition>,
		byte_size: i64,
		duration: f32,
	) -> pb::scuffle::video::v1::types::Recording {
		pb::scuffle::video::v1::types::Recording {
			id: Some(self.id.0.into()),
			created_at: self.id.0.timestamp_ms() as i64,
			deleted_at: self.deleted_at.map(|dt| dt.timestamp_millis()),
			room_id: self.room_id.map(|id| id.0.into()),
			recording_config_id: self.recording_config_id.map(|id| id.0.into()),
			s3_bucket_id: Some(self.s3_bucket_id.0.into()),
			updated_at: self.updated_at.timestamp_millis(),
			tags: Some(self.tags.into()),
			visibility: self.visibility.into(),
			ended_at: self.ended_at.map(|dt| dt.timestamp_millis()),
			renditions: renditions
				.into_iter()
				.map(|r| pb::scuffle::video::v1::types::Rendition::from(r) as i32)
				.collect(),
			byte_size,
			duration,
		}
	}
}
