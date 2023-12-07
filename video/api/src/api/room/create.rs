use std::sync::Arc;

use pb::scuffle::video::v1::events_fetch_request::Target;
use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::{event, Resource};
use pb::scuffle::video::v1::{RoomCreateRequest, RoomCreateResponse};
use tonic::Status;
use ulid::Ulid;
use video_common::database::{AccessToken, DatabaseTable, RecordingConfig, TranscodingConfig, Visibility};

use super::utils::create_stream_key;
use crate::api::utils::tags::validate_tags;
use crate::api::utils::{events, impl_request_scopes, ApiRequest, TonicRequest};
use crate::global::ApiGlobal;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	RoomCreateRequest,
	video_common::database::Room,
	(Resource::Room, Permission::Create),
	RateLimitResource::RoomCreate
);

pub fn validate(req: &RoomCreateRequest) -> tonic::Result<()> {
	validate_tags(req.tags.as_ref())
}

pub async fn build_query<G: ApiGlobal>(
	req: &RoomCreateRequest,
	global: &Arc<G>,
	access_token: &AccessToken,
) -> tonic::Result<sqlx::QueryBuilder<'static, sqlx::Postgres>> {
	let mut qb = sqlx::query_builder::QueryBuilder::default();

	qb.push("INSERT INTO ")
		.push(<RoomCreateRequest as TonicRequest>::Table::NAME)
		.push(" (");

	let mut seperated = qb.separated(",");

	seperated.push("id");
	seperated.push("organization_id");
	seperated.push("transcoding_config_id");
	seperated.push("recording_config_id");
	seperated.push("visibility");
	seperated.push("stream_key");
	seperated.push("tags");

	qb.push(") VALUES (");

	let transcoding_config: Option<TranscodingConfig> = if let Some(transcoding_config_id) = &req.transcoding_config_id {
		Some(
			sqlx::query_as("SELECT * FROM transcoding_configs WHERE id = $1 AND organization_id = $2")
				.bind(common::database::Ulid(transcoding_config_id.into_ulid()))
				.bind(access_token.organization_id)
				.fetch_optional(global.db().as_ref())
				.await
				.map_err(|err| {
					tracing::error!(err = %err, "failed to fetch transcoding config");
					Status::internal("failed to fetch transcoding config")
				})?
				.ok_or_else(|| Status::not_found("transcoding config not found"))?,
		)
	} else {
		None
	};

	let recording_config: Option<RecordingConfig> = if let Some(recording_config_id) = &req.recording_config_id {
		Some(
			sqlx::query_as("SELECT * FROM recording_configs WHERE id = $1 AND organization_id = $2")
				.bind(common::database::Ulid(recording_config_id.into_ulid()))
				.bind(access_token.organization_id)
				.fetch_optional(global.db().as_ref())
				.await
				.map_err(|err| {
					tracing::error!(err = %err, "failed to fetch recording config");
					Status::internal("failed to fetch recording config")
				})?
				.ok_or_else(|| Status::not_found("recording config not found"))?,
		)
	} else {
		None
	};

	let visibility = pb::scuffle::video::v1::types::Visibility::try_from(req.visibility)
		.map_err(|_| Status::invalid_argument("invalid visibility value"))?;

	// The stream key is 32 characters long randomly generated string.
	let mut seperated = qb.separated(",");

	seperated.push_bind(common::database::Ulid(Ulid::new()));
	seperated.push_bind(access_token.organization_id);
	seperated.push_bind(transcoding_config.map(|t| t.id));
	seperated.push_bind(recording_config.map(|r| r.id));
	seperated.push_bind(Visibility::from(visibility));
	seperated.push_bind(create_stream_key());
	seperated.push_bind(sqlx::types::Json(req.tags.clone().unwrap_or_default().tags));

	qb.push(") RETURNING *");

	Ok(qb)
}

#[async_trait::async_trait]
impl ApiRequest<RoomCreateResponse> for tonic::Request<RoomCreateRequest> {
	async fn process<G: ApiGlobal>(
		&self,
		global: &Arc<G>,
		access_token: &AccessToken,
	) -> tonic::Result<tonic::Response<RoomCreateResponse>> {
		let req = self.get_ref();

		validate(req)?;

		let mut query = build_query(req, global, access_token).await?;

		let result: video_common::database::Room =
			query.build_query_as().fetch_one(global.db().as_ref()).await.map_err(|err| {
				tracing::error!(err = %err, "failed to create {}", <RoomCreateRequest as TonicRequest>::Table::FRIENDLY_NAME);
				tonic::Status::internal(format!(
					"failed to create {}",
					<RoomCreateRequest as TonicRequest>::Table::FRIENDLY_NAME
				))
			})?;

		events::emit(
			global,
			access_token.organization_id.0,
			Target::Room,
			event::Event::Room(event::Room {
				room_id: Some(result.id.0.into()),
				event: Some(event::room::Event::Created(event::room::Created {})),
			}),
		)
		.await;

		Ok(tonic::Response::new(RoomCreateResponse {
			stream_key: result.stream_key.clone(),
			room: Some(result.into_proto()),
		}))
	}
}
