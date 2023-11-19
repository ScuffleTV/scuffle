use std::sync::Arc;

use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::Resource;
use pb::scuffle::video::v1::{PlaybackKeyPairCreateRequest, PlaybackKeyPairCreateResponse};
use tonic::Status;
use ulid::Ulid;
use video_common::database::{AccessToken, DatabaseTable};

use super::utils::validate_public_key;
use crate::api::utils::tags::validate_tags;
use crate::api::utils::{impl_request_scopes, QbRequest, QbResponse, TonicRequest};
use crate::global::ApiGlobal;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	PlaybackKeyPairCreateRequest,
	video_common::database::PlaybackKeyPair,
	(Resource::PlaybackKeyPair, Permission::Create),
	RateLimitResource::PlaybackKeyPairCreate
);

#[async_trait::async_trait]
impl QbRequest for PlaybackKeyPairCreateRequest {
	type QueryObject = Self::Table;

	async fn build_query<G: ApiGlobal>(
		&self,
		_: &Arc<G>,
		access_token: &AccessToken,
	) -> tonic::Result<sqlx::QueryBuilder<'_, sqlx::Postgres>> {
		validate_tags(self.tags.as_ref())?;

		let (cert, fingerprint) = validate_public_key(&self.public_key)?;

		let mut qb = sqlx::query_builder::QueryBuilder::default();

		qb.push("INSERT INTO ").push(Self::Table::NAME).push(" (");

		let mut seperated = qb.separated(",");

		seperated.push("id");
		seperated.push("organization_id");
		seperated.push("public_key");
		seperated.push("fingerprint");
		seperated.push("updated_at");
		seperated.push("tags");

		qb.push(") VALUES (");

		let mut seperated = qb.separated(",");

		seperated.push_bind(common::database::Ulid(Ulid::new()));
		seperated.push_bind(access_token.organization_id);
		seperated.push_bind(cert);
		seperated.push_bind(fingerprint);
		seperated.push_bind(chrono::Utc::now());
		seperated.push_bind(sqlx::types::Json(self.tags.clone().unwrap_or_default().tags));

		qb.push(") RETURNING *");

		Ok(qb)
	}
}

impl QbResponse for PlaybackKeyPairCreateResponse {
	type Request = PlaybackKeyPairCreateRequest;

	fn from_query_object(query_object: Vec<<Self::Request as QbRequest>::QueryObject>) -> tonic::Result<Self> {
		if query_object.is_empty() {
			return Err(Status::internal(format!(
				"failed to create {}, no rows returned",
				<Self::Request as TonicRequest>::Table::FRIENDLY_NAME
			)));
		}

		Ok(Self {
			playback_key_pair: Some(query_object.into_iter().next().unwrap().into_proto()),
		})
	}
}
