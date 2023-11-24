use std::collections::HashSet;
use std::sync::Arc;

use pb::ext::UlidExt;
use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::Resource;
use pb::scuffle::video::v1::{RecordingConfigModifyRequest, RecordingConfigModifyResponse};
use tonic::Status;
use video_common::database::{AccessToken, DatabaseTable, Rendition, S3Bucket};

use crate::api::errors::MODIFY_NO_FIELDS;
use crate::api::utils::tags::validate_tags;
use crate::api::utils::{impl_request_scopes, QbRequest, QbResponse, TonicRequest};
use crate::global::ApiGlobal;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	RecordingConfigModifyRequest,
	video_common::database::RecordingConfig,
	(Resource::RecordingConfig, Permission::Modify),
	RateLimitResource::RecordingConfigModify
);

#[async_trait::async_trait]
impl QbRequest for RecordingConfigModifyRequest {
	type QueryObject = Self::Table;

	async fn build_query<G: ApiGlobal>(
		&self,
		global: &Arc<G>,
		access_token: &AccessToken,
	) -> tonic::Result<sqlx::QueryBuilder<'_, sqlx::Postgres>> {
		validate_tags(self.tags.as_ref())?;

		let mut qb = sqlx::query_builder::QueryBuilder::default();

		qb.push("UPDATE ")
			.push(<RecordingConfigModifyRequest as TonicRequest>::Table::NAME)
			.push(" SET ");

		let mut seperated = qb.separated(",");

		if let Some(renditions) = &self.stored_renditions {
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

		if let Some(lifecycle_policies) = &self.lifecycle_policies {
			seperated.push("lifecycle_policies = ").push_bind_unseparated(
				lifecycle_policies
					.items
					.clone()
					.into_iter()
					.map(common::database::Protobuf)
					.collect::<Vec<_>>(),
			);
		}

		if let Some(tags) = &self.tags {
			seperated.push("tags = ").push_bind_unseparated(sqlx::types::Json(&tags.tags));
		}

		if let Some(s3_bucket_id) = &self.s3_bucket_id {
			let bucket: S3Bucket = sqlx::query_as("SELECT * FROM s3_buckets WHERE id = $1 AND organization_id = $2")
				.bind(common::database::Ulid(s3_bucket_id.into_ulid()))
				.bind(access_token.organization_id)
				.fetch_optional(global.db().as_ref())
				.await
				.map_err(|err| {
					tracing::error!(err = %err, "failed to fetch s3 bucket");
					Status::internal("failed to fetch s3 bucket")
				})?
				.ok_or_else(|| Status::not_found("s3 bucket not found"))?;

			seperated.push("s3_bucket_id = ").push_bind_unseparated(bucket.id);
		}

		if self.tags.is_none()
			&& self.stored_renditions.is_none()
			&& self.lifecycle_policies.is_none()
			&& self.s3_bucket_id.is_none()
		{
			return Err(Status::invalid_argument(MODIFY_NO_FIELDS));
		}

		seperated.push("updated_at = NOW()");

		qb.push(" WHERE id = ").push_bind(common::database::Ulid(self.id.into_ulid()));
		qb.push(" AND organization_id = ").push_bind(access_token.organization_id);
		qb.push(" RETURNING *");

		Ok(qb)
	}
}

impl QbResponse for RecordingConfigModifyResponse {
	type Request = RecordingConfigModifyRequest;

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
			recording_config: Some(query_object.into_iter().next().unwrap().into_proto()),
		})
	}
}
