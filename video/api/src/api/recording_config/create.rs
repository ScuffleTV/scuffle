use std::collections::HashSet;
use std::sync::Arc;

use pb::scuffle::video::v1::events_fetch_request::Target;
use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::{event, Resource};
use pb::scuffle::video::v1::{RecordingConfigCreateRequest, RecordingConfigCreateResponse};
use tonic::Status;
use ulid::Ulid;
use video_common::database::{AccessToken, DatabaseTable, Rendition, S3Bucket};

use crate::api::utils::tags::validate_tags;
use crate::api::utils::{events, impl_request_scopes, ApiRequest, TonicRequest};
use crate::global::ApiGlobal;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	RecordingConfigCreateRequest,
	video_common::database::RecordingConfig,
	(Resource::RecordingConfig, Permission::Create),
	RateLimitResource::RecordingConfigCreate
);

pub fn validate(req: &RecordingConfigCreateRequest) -> tonic::Result<()> {
	validate_tags(req.tags.as_ref())
}

pub async fn build_query<G: ApiGlobal>(
	req: &RecordingConfigCreateRequest,
	global: &Arc<G>,
	access_token: &AccessToken,
) -> tonic::Result<sqlx::QueryBuilder<'static, sqlx::Postgres>> {
	let mut qb = sqlx::query_builder::QueryBuilder::default();

	qb.push("INSERT INTO ")
		.push(<RecordingConfigCreateRequest as TonicRequest>::Table::NAME)
		.push(" (");

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

	let renditions = req.stored_renditions().map(Rendition::from).collect::<HashSet<_>>();

	if !renditions.iter().any(|r| r.is_audio()) {
		return Err(Status::invalid_argument("must specify at least one audio rendition"));
	}

	if !renditions.iter().any(|r| r.is_video()) {
		return Err(Status::invalid_argument("must specify at least one video rendition"));
	}

	let bucket: S3Bucket = if let Some(s3_bucket_id) = &req.s3_bucket_id {
		sqlx::query_as("SELECT * FROM s3_buckets WHERE id = $1 AND organization_id = $2")
			.bind(common::database::Ulid(s3_bucket_id.into_ulid()))
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
		req.lifecycle_policies
			.clone()
			.into_iter()
			.map(common::database::Protobuf)
			.collect::<Vec<_>>(),
	);
	seperated.push_bind(chrono::Utc::now());
	seperated.push_bind(bucket.id);
	seperated.push_bind(sqlx::types::Json(req.tags.clone().unwrap_or_default().tags));

	qb.push(") RETURNING *");

	Ok(qb)
}

#[async_trait::async_trait]
impl ApiRequest<RecordingConfigCreateResponse> for tonic::Request<RecordingConfigCreateRequest> {
	async fn process<G: ApiGlobal>(
		&self,
		global: &Arc<G>,
		access_token: &AccessToken,
	) -> tonic::Result<tonic::Response<RecordingConfigCreateResponse>> {
		let req = self.get_ref();

		validate(req)?;

		let mut query = build_query(req, global, access_token).await?;

		let result: video_common::database::RecordingConfig =
			query.build_query_as().fetch_one(global.db().as_ref()).await.map_err(|err| {
				tracing::error!(err = %err, "failed to create {}", <RecordingConfigCreateRequest as TonicRequest>::Table::FRIENDLY_NAME);
				Status::internal(format!(
					"failed to create {}",
					<RecordingConfigCreateRequest as TonicRequest>::Table::FRIENDLY_NAME
				))
			})?;

		events::emit(
			global,
			access_token.organization_id.0,
			Target::RecordingConfig,
			event::Event::RecordingConfig(event::RecordingConfig {
				recording_config_id: Some(result.id.0.into()),
				event: Some(event::recording_config::Event::Created(event::recording_config::Created {})),
			}),
		)
		.await;

		Ok(tonic::Response::new(RecordingConfigCreateResponse {
			recording_config: Some(result.into_proto()),
		}))
	}
}
