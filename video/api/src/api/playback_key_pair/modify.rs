use std::sync::Arc;

use pb::ext::UlidExt;
use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::Resource;
use pb::scuffle::video::v1::{PlaybackKeyPairModifyRequest, PlaybackKeyPairModifyResponse};
use video_common::database::{AccessToken, DatabaseTable};

use super::utils::validate_public_key;
use crate::api::utils::tags::validate_tags;
use crate::api::utils::{impl_request_scopes, QbRequest, QbResponse, TonicRequest};
use crate::global::ApiGlobal;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	PlaybackKeyPairModifyRequest,
	video_common::database::PlaybackKeyPair,
	(Resource::PlaybackKeyPair, Permission::Modify),
	RateLimitResource::PlaybackKeyPairModify
);

#[async_trait::async_trait]
impl QbRequest for PlaybackKeyPairModifyRequest {
	type QueryObject = Self::Table;

	async fn build_query<G: ApiGlobal>(
		&self,
		_global: &Arc<G>,
		access_token: &AccessToken,
	) -> tonic::Result<sqlx::QueryBuilder<'_, sqlx::Postgres>> {
		validate_tags(self.tags.as_ref())?;

		let mut qb = sqlx::query_builder::QueryBuilder::default();

		qb.push("UPDATE ").push(Self::Table::NAME).push(" SET ");

		let mut seperated = qb.separated(",");

		if let Some(tags) = &self.tags {
			seperated
				.push("tags = ")
				.push_bind_unseparated(sqlx::types::Json(tags.tags.clone()));
		}

		if let Some(public_key) = &self.public_key {
			let (cert, fingerprint) = validate_public_key(public_key)?;

			seperated.push("public_key = ").push_bind_unseparated(cert);
			seperated.push("fingerprint = ").push_bind_unseparated(fingerprint);
		}

		seperated.push("updated_at = ").push_bind(chrono::Utc::now());

		qb.push(" WHERE id = ").push_bind(self.id.to_uuid());
		qb.push(" AND organization_id = ").push_bind(access_token.organization_id);
		qb.push(" RETURNING *");

		Ok(qb)
	}
}

impl QbResponse for PlaybackKeyPairModifyResponse {
	type Request = PlaybackKeyPairModifyRequest;

	fn from_query_object(query_object: Vec<<Self::Request as QbRequest>::QueryObject>) -> tonic::Result<Self> {
		if query_object.is_empty() {
			return Err(tonic::Status::not_found(format!(
				"{} not found",
				<<Self::Request as TonicRequest>::Table as DatabaseTable>::FRIENDLY_NAME
			)));
		}

		if query_object.len() > 1 {
			return Err(tonic::Status::internal(format!(
				"failed to modify {}, {} rows returned",
				<<Self::Request as TonicRequest>::Table as DatabaseTable>::FRIENDLY_NAME,
				query_object.len(),
			)));
		}

		Ok(Self {
			playback_key_pair: Some(query_object.into_iter().next().unwrap().into_proto()),
		})
	}
}
