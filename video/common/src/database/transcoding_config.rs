use std::collections::HashMap;

use common::database::Ulid;
use pb::scuffle::video::v1::types::Rendition as PbRendition;

use super::Rendition;

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct TranscodingConfig {
	pub id: Ulid,
	pub organization_id: Ulid,
	pub renditions: Vec<Rendition>,
	pub updated_at: chrono::DateTime<chrono::Utc>,
	pub tags: sqlx::types::Json<HashMap<String, String>>,
}

impl TranscodingConfig {
	pub fn into_proto(self) -> pb::scuffle::video::v1::types::TranscodingConfig {
		pb::scuffle::video::v1::types::TranscodingConfig {
			id: Some(self.id.0.into()),
			renditions: self.renditions.into_iter().map(|r| PbRendition::from(r).into()).collect(),
			created_at: self.id.0.timestamp_ms() as i64,
			updated_at: self.updated_at.timestamp_micros(),
			tags: Some(self.tags.0.into()),
		}
	}
}
