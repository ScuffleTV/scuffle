use std::collections::HashSet;
use std::sync::Arc;

use pb::ext::UlidExt;
use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::Resource;
use pb::scuffle::video::v1::{RecordingConfigCreateRequest, RecordingConfigCreateResponse};
use tonic::Status;
use ulid::Ulid;
use video_common::database::{AccessToken, DatabaseTable, Rendition, S3Bucket};

use crate::api::utils::tags::validate_tags;
use crate::api::utils::{impl_request_scopes, QbRequest, QbResponse, TonicRequest};
use crate::global::ApiGlobal;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	RecordingConfigCreateRequest,
	video_common::database::RecordingConfig,
	(Resource::RecordingConfig, Permission::Create),
	RateLimitResource::RecordingConfigCreate
);

#[async_trait::async_trait]
impl QbRequest for RecordingConfigCreateRequest {
	type QueryObject = Self::Table;

	async fn build_query<G: ApiGlobal>(
		&self,
		global: &Arc<G>,
		access_token: &AccessToken,
	) -> tonic::Result<sqlx::QueryBuilder<'_, sqlx::Postgres>> {
		validate_tags(self.tags.as_ref())?;

		let mut qb = sqlx::query_builder::QueryBuilder::default();

		qb.push("INSERT INTO ").push(Self::Table::NAME).push(" (");

		let mut seperated = qb.separated(",");

		seperated.push("id");
		seperated.push("organization_id");
		seperated.push("renditions");
		seperated.push("lifecycle_policies");
		seperated.push("updated_at");
		seperated.push("s3_bucket_id");
		seperated.push("tags");

		qb.push(") VALUES (");

		let mut seperated = qb.separated(",");

		let renditions = self.stored_renditions().map(Rendition::from).collect::<HashSet<_>>();

		if !renditions.iter().any(|r| r.is_audio()) {
			return Err(Status::invalid_argument("must specify at least one audio rendition"));
		}

		if !renditions.iter().any(|r| r.is_video()) {
			return Err(Status::invalid_argument("must specify at least one video rendition"));
		}

		let bucket: S3Bucket = if let Some(s3_bucket_id) = &self.s3_bucket_id {
			sqlx::query_as("SELECT * FROM s3_buckets WHERE id = $1 AND organization_id = $2")
				.bind(common::database::Ulid(s3_bucket_id.to_ulid()))
				.bind(access_token.organization_id)
				.fetch_optional(global.db().as_ref())
				.await
		} else {
			sqlx::query_as("SELECT * FROM s3_buckets WHERE organization_id = $1 AND managed = TRUE LIMIT 1")
				.bind(access_token.organization_id)
				.fetch_optional(global.db().as_ref())
				.await
		}
		.map_err(|err| {
			tracing::error!(err = %err, "failed to fetch s3 bucket");
			Status::internal("failed to fetch s3 bucket")
		})?
		.ok_or_else(|| Status::not_found("s3 bucket not found"))?;

		seperated.push_bind(common::database::Ulid(Ulid::new()));
		seperated.push_bind(access_token.organization_id);
		seperated.push_bind(renditions.into_iter().collect::<Vec<_>>());
		seperated.push_bind(
			self.lifecycle_policies
				.clone()
				.into_iter()
				.map(common::database::Protobuf)
				.collect::<Vec<_>>(),
		);
		seperated.push_bind(chrono::Utc::now());
		seperated.push_bind(bucket.id);
		seperated.push_bind(sqlx::types::Json(self.tags.clone().unwrap_or_default().tags));

		qb.push(") RETURNING *");

		Ok(qb)
	}
}

impl QbResponse for RecordingConfigCreateResponse {
	type Request = RecordingConfigCreateRequest;

	fn from_query_object(query_object: Vec<<Self::Request as QbRequest>::QueryObject>) -> tonic::Result<Self> {
		if query_object.is_empty() {
			return Err(Status::internal(format!(
				"failed to create {}, no rows returned",
				<Self::Request as TonicRequest>::Table::FRIENDLY_NAME
			)));
		}

		Ok(Self {
			recording_config: Some(query_object.into_iter().next().unwrap().into_proto()),
		})
	}
}
