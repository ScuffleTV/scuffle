use std::collections::HashMap;

use chrono::Utc;
use common::database::{Protobuf, Ulid};
use pb::scuffle::video::v1::types::AccessTokenScope;

use super::DatabaseTable;

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct AccessToken {
	pub id: Ulid,
	pub organization_id: Ulid,
	pub secret_key: Ulid,
	pub last_active_at: Option<chrono::DateTime<Utc>>,
	pub updated_at: chrono::DateTime<Utc>,
	pub expires_at: Option<chrono::DateTime<Utc>>,
	pub scopes: Vec<Protobuf<AccessTokenScope>>,
	pub tags: sqlx::types::Json<HashMap<String, String>>,
}

impl DatabaseTable for AccessToken {
	const FRIENDLY_NAME: &'static str = "access token";
	const NAME: &'static str = "access_tokens";
}

impl AccessToken {
	pub fn into_proto(self) -> pb::scuffle::video::v1::types::AccessToken {
		pb::scuffle::video::v1::types::AccessToken {
			id: Some(self.id.0.into()),
			created_at: self.id.0.timestamp_ms() as i64,
			updated_at: self.updated_at.timestamp_millis(),
			expires_at: self.expires_at.map(|t| t.timestamp_millis()),
			last_used_at: self.last_active_at.map(|t| t.timestamp_millis()),
			scopes: self.scopes.into_iter().map(|s| s.0).collect(),
			tags: Some(self.tags.0.into()),
		}
	}
}
