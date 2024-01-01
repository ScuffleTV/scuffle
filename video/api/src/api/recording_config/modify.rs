use std::collections::HashSet;
use std::sync::Arc;

use pb::ext::UlidExt;
use pb::scuffle::video::v1::events_fetch_request::Target;
use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::{event, Resource};
use pb::scuffle::video::v1::{RecordingConfigModifyRequest, RecordingConfigModifyResponse};
use tonic::Status;
use video_common::database::{AccessToken, DatabaseTable, Rendition, S3Bucket};

use crate::api::errors::MODIFY_NO_FIELDS;
use crate::api::utils::tags::validate_tags;
use crate::api::utils::{impl_request_scopes, ApiRequest, TonicRequest};
use crate::global::ApiGlobal;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	RecordingConfigModifyRequest,
	video_common::database::RecordingConfig,
	(Resource::RecordingConfig, Permission::Modify),
	RateLimitResource::RecordingConfigModify
);

pub fn validate(req: &RecordingConfigModifyRequest) -> tonic::Result<()> {
	validate_tags(req.tags.as_ref())
}

pub async fn build_query<'a, G: ApiGlobal>(
	req: &'a RecordingConfigModifyRequest,
	global: &'a Arc<G>,
	access_token: &AccessToken,
) -> tonic::Result<sqlx::QueryBuilder<'a, sqlx::Postgres>> {
	let mut qb = sqlx::query_builder::QueryBuilder::default();

	qb.push("UPDATE ")
		.push(<RecordingConfigModifyRequest as TonicRequest>::Table::NAME)
		.push(" SET ");

	let mut seperated = qb.separated(",");

	if let Some(renditions) = &req.stored_renditions {
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

	if let Some(lifecycle_policies) = &req.lifecycle_policies {
		seperated.push("lifecycle_policies = ").push_bind_unseparated(
			lifecycle_policies
				.items
				.clone()
				.into_iter()
				.map(common::database::Protobuf)
				.collect::<Vec<_>>(),
		);
	}

	if let Some(tags) = &req.tags {
		seperated.push("tags = ").push_bind_unseparated(sqlx::types::Json(&tags.tags));
	}

	if let Some(s3_bucket_id) = &req.s3_bucket_id {
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

	if req.tags.is_none()
		&& req.stored_renditions.is_none()
		&& req.lifecycle_policies.is_none()
		&& req.s3_bucket_id.is_none()
	{
		return Err(Status::invalid_argument(MODIFY_NO_FIELDS));
	}

	seperated.push("updated_at = NOW()");

	qb.push(" WHERE id = ").push_bind(common::database::Ulid(req.id.into_ulid()));
	qb.push(" AND organization_id = ").push_bind(access_token.organization_id);
	qb.push(" RETURNING *");

	Ok(qb)
}

impl ApiRequest<RecordingConfigModifyResponse> for tonic::Request<RecordingConfigModifyRequest> {
	async fn process<G: ApiGlobal>(
		&self,
		global: &Arc<G>,
		access_token: &AccessToken,
	) -> tonic::Result<tonic::Response<RecordingConfigModifyResponse>> {
		let req = self.get_ref();

		validate(req)?;

		let mut query = build_query(req, global, access_token).await?;

		let result: Option<video_common::database::RecordingConfig> = query
			.build_query_as()
			.fetch_optional(global.db().as_ref())
			.await
			.map_err(|err| {
				tracing::error!(err = %err, "failed to modify {}", <RecordingConfigModifyRequest as TonicRequest>::Table::FRIENDLY_NAME);
				tonic::Status::internal(format!(
					"failed to modify {}",
					<RecordingConfigModifyRequest as TonicRequest>::Table::FRIENDLY_NAME
				))
			})?;

		match result {
			Some(result) => {
				video_common::events::emit(
					global.nats(),
					access_token.organization_id.0,
					Target::RecordingConfig,
					event::Event::RecordingConfig(event::RecordingConfig {
						recording_config_id: Some(result.id.0.into()),
						event: Some(event::recording_config::Event::Modified(event::recording_config::Modified {})),
					}),
				)
				.await;
				Ok(tonic::Response::new(RecordingConfigModifyResponse {
					recording_config: Some(result.into_proto()),
				}))
			}
			None => Err(tonic::Status::not_found(format!(
				"{} not found",
				<RecordingConfigModifyRequest as TonicRequest>::Table::FRIENDLY_NAME
			))),
		}
	}
}
