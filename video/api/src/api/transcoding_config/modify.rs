use std::collections::HashSet;
use std::sync::Arc;

use pb::ext::UlidExt;
use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::Resource;
use pb::scuffle::video::v1::{TranscodingConfigModifyRequest, TranscodingConfigModifyResponse};
use tonic::Status;
use video_common::database::{AccessToken, DatabaseTable, Rendition};

use crate::api::errors::MODIFY_NO_FIELDS;
use crate::api::utils::tags::validate_tags;
use crate::api::utils::{impl_request_scopes, QbRequest, QbResponse, TonicRequest};
use crate::global::ApiGlobal;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	TranscodingConfigModifyRequest,
	video_common::database::TranscodingConfig,
	(Resource::TranscodingConfig, Permission::Modify),
	RateLimitResource::TranscodingConfigModify
);

#[async_trait::async_trait]
impl QbRequest for TranscodingConfigModifyRequest {
	type QueryObject = Self::Table;

	async fn build_query<G: ApiGlobal>(
		&self,
		_: &Arc<G>,
		access_token: &AccessToken,
	) -> tonic::Result<sqlx::QueryBuilder<'_, sqlx::Postgres>> {
		validate_tags(self.tags.as_ref())?;

		let mut qb = sqlx::query_builder::QueryBuilder::default();

		qb.push("UPDATE ").push(Self::Table::NAME).push(" SET ");

		let mut seperated = qb.separated(",");

		if let Some(renditions) = &self.renditions {
			let renditions = renditions.items().map(Rendition::from).collect::<HashSet<_>>();

			if !renditions.iter().any(|r| r.is_audio()) {
				return Err(Status::invalid_argument("must specify at least one audio rendition"));
			}

			if !renditions.iter().any(|r| r.is_video()) {
				return Err(Status::invalid_argument("must specify at least one video rendition"));
			}

			seperated
				.push("renditions = ")
				.push_bind_unseparated(renditions.into_iter().collect::<Vec<_>>());
		}

		if let Some(tags) = &self.tags {
			seperated.push("tags = ").push_bind_unseparated(sqlx::types::Json(&tags.tags));
		}

		if self.renditions.is_none() && self.tags.is_none() {
			return Err(tonic::Status::invalid_argument(MODIFY_NO_FIELDS));
		}

		seperated.push("updated_at = NOW()");

		qb.push(" WHERE id = ").push_bind(common::database::Ulid(self.id.into_ulid()));
		qb.push(" AND organization_id = ").push_bind(access_token.organization_id);
		qb.push(" RETURNING *");

		Ok(qb)
	}
}

impl QbResponse for TranscodingConfigModifyResponse {
	type Request = TranscodingConfigModifyRequest;

	fn from_query_object(query_object: Vec<<Self::Request as QbRequest>::QueryObject>) -> tonic::Result<Self> {
		if query_object.is_empty() {
			return Err(tonic::Status::not_found(format!(
				"{} not found",
				<Self::Request as TonicRequest>::Table::FRIENDLY_NAME
			)));
		}

		if query_object.len() > 1 {
			return Err(tonic::Status::internal(format!(
				"failed to modify {}, {} rows returned",
				<Self::Request as TonicRequest>::Table::FRIENDLY_NAME,
				query_object.len(),
			)));
		}

		Ok(Self {
			transcoding_config: Some(query_object.into_iter().next().unwrap().into_proto()),
		})
	}
}
