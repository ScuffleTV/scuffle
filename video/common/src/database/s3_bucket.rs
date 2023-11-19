use std::collections::HashMap;

use common::database::Ulid;

use super::DatabaseTable;

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct S3Bucket {
	pub id: Ulid,
	pub organization_id: Ulid,

	pub name: String,
	pub region: String,
	pub endpoint: Option<String>,
	pub access_key_id: String,
	pub secret_access_key: String,
	pub public_url: Option<String>,
	pub managed: bool,

	pub tags: sqlx::types::Json<HashMap<String, String>>,
}

impl DatabaseTable for S3Bucket {
	const FRIENDLY_NAME: &'static str = "s3 bucket";
	const NAME: &'static str = "s3_buckets";
}

impl S3Bucket {
	pub fn into_proto(self) -> pb::scuffle::video::v1::types::S3Bucket {
		pb::scuffle::video::v1::types::S3Bucket {
			id: Some(self.id.0.into()),
			name: self.name,
			region: self.region,
			endpoint: self.endpoint,
			access_key_id: self.access_key_id,
			public_url: self.public_url,
			managed: self.managed,
			tags: Some(self.tags.0.into()),
		}
	}
}
