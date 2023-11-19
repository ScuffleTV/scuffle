use std::collections::HashMap;

use common::database::Ulid;

use super::{DatabaseTable, Rendition};

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct Recording {
	pub id: Ulid,
	pub organization_id: Ulid,

	pub room_id: Option<Ulid>,
	pub recording_config_id: Option<Ulid>,
	pub s3_bucket_id: Ulid,

	pub public: bool,
	pub allow_dvr: bool,

	pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
	pub updated_at: chrono::DateTime<chrono::Utc>,
	pub ended_at: Option<chrono::DateTime<chrono::Utc>>,

	pub tags: sqlx::types::Json<HashMap<String, String>>,
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
			tags: Some(self.tags.0.into()),
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
