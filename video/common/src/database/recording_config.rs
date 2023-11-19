use std::collections::HashMap;

use common::database::{Protobuf, Ulid};
use pb::scuffle::video::v1::types::{RecordingLifecyclePolicy, Rendition as PbRendition};

use super::{DatabaseTable, Rendition};

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct RecordingConfig {
	pub id: Ulid,
	pub organization_id: Ulid,
	pub renditions: Vec<Rendition>,
	pub lifecycle_policies: Vec<Protobuf<RecordingLifecyclePolicy>>,
	pub updated_at: chrono::DateTime<chrono::Utc>,
	pub s3_bucket_id: Ulid,
	pub tags: sqlx::types::Json<HashMap<String, String>>,
}

impl DatabaseTable for RecordingConfig {
	const FRIENDLY_NAME: &'static str = "recording config";
	const NAME: &'static str = "recording_configs";
}

impl RecordingConfig {
	pub fn into_proto(self) -> pb::scuffle::video::v1::types::RecordingConfig {
		pb::scuffle::video::v1::types::RecordingConfig {
			id: Some(self.id.0.into()),
			renditions: self.renditions.into_iter().map(|r| PbRendition::from(r).into()).collect(),
			s3_bucket_id: Some(self.s3_bucket_id.0.into()),
			lifecycle_policies: self.lifecycle_policies.into_iter().map(|p| p.0).collect(),
			created_at: self.id.0.timestamp_ms() as i64,
			updated_at: self.updated_at.timestamp_millis(),
			tags: Some(self.tags.0.into()),
		}
	}
}
