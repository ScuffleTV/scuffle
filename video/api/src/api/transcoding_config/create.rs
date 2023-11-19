use std::collections::HashSet;
use std::sync::Arc;

use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::Resource;
use pb::scuffle::video::v1::{TranscodingConfigCreateRequest, TranscodingConfigCreateResponse};
use tonic::Status;
use ulid::Ulid;
use video_common::database::{AccessToken, DatabaseTable, Rendition};

use crate::api::utils::tags::validate_tags;
use crate::api::utils::{impl_request_scopes, QbRequest, QbResponse, TonicRequest};
use crate::global::ApiGlobal;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	TranscodingConfigCreateRequest,
	video_common::database::TranscodingConfig,
	(Resource::TranscodingConfig, Permission::Create),
	RateLimitResource::TranscodingConfigCreate
);

#[async_trait::async_trait]
impl QbRequest for TranscodingConfigCreateRequest {
	type QueryObject = Self::Table;

	async fn build_query<G: ApiGlobal>(
		&self,
		_: &Arc<G>,
		access_token: &AccessToken,
	) -> tonic::Result<sqlx::QueryBuilder<'_, sqlx::Postgres>> {
		validate_tags(self.tags.as_ref())?;

		let mut qb = sqlx::query_builder::QueryBuilder::default();

		qb.push("INSERT INTO ").push(Self::Table::NAME).push(" (");

		let mut seperated = qb.separated(",");

		seperated.push("id");
		seperated.push("organization_id");
		seperated.push("renditions");
		seperated.push("tags");

		qb.push(") VALUES (");

		let mut seperated = qb.separated(",");

		let renditions = self.renditions().map(Rendition::from).collect::<HashSet<_>>();

		if !renditions.iter().any(|r| r.is_audio()) {
			return Err(Status::invalid_argument("must specify at least one audio rendition"));
		}

		if !renditions.iter().any(|r| r.is_video()) {
			return Err(Status::invalid_argument("must specify at least one video rendition"));
		}

		seperated.push_bind(common::database::Ulid(Ulid::new()));
		seperated.push_bind(access_token.organization_id);
		seperated.push_bind(renditions.into_iter().collect::<Vec<_>>());
		seperated.push_bind(sqlx::types::Json(self.tags.clone().unwrap_or_default().tags));

		qb.push(") RETURNING *");

		Ok(qb)
	}
}

impl QbResponse for TranscodingConfigCreateResponse {
	type Request = TranscodingConfigCreateRequest;

	fn from_query_object(query_object: Vec<<Self::Request as QbRequest>::QueryObject>) -> tonic::Result<Self> {
		if query_object.is_empty() {
			return Err(Status::internal(format!(
				"failed to create {}, no rows returned",
				<Self::Request as TonicRequest>::Table::FRIENDLY_NAME
			)));
		}

		Ok(Self {
			transcoding_config: Some(query_object.into_iter().next().unwrap().into_proto()),
		})
	}
}
