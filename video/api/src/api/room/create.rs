use std::sync::Arc;

use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::Resource;
use pb::scuffle::video::v1::{RoomCreateRequest, RoomCreateResponse};
use tonic::Status;
use ulid::Ulid;
use video_common::database::{AccessToken, DatabaseTable, RecordingConfig, TranscodingConfig, Visibility};

use super::utils::create_stream_key;
use crate::api::utils::tags::validate_tags;
use crate::api::utils::{impl_request_scopes, QbRequest, QbResponse, TonicRequest};
use crate::global::ApiGlobal;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	RoomCreateRequest,
	video_common::database::Room,
	(Resource::Room, Permission::Create),
	RateLimitResource::RoomCreate
);

#[async_trait::async_trait]
impl QbRequest for RoomCreateRequest {
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
		seperated.push("transcoding_config_id");
		seperated.push("recording_config_id");
		seperated.push("visibility");
		seperated.push("stream_key");
		seperated.push("tags");

		qb.push(") VALUES (");

		let transcoding_config: Option<TranscodingConfig> = if let Some(transcoding_config_id) = &self.transcoding_config_id
		{
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

		let recording_config: Option<RecordingConfig> = if let Some(recording_config_id) = &self.recording_config_id {
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

		let visibility = pb::scuffle::video::v1::types::Visibility::try_from(self.visibility)
			.map_err(|_| Status::invalid_argument("invalid visibility value"))?;

		// The stream key is 32 characters long randomly generated string.
		let mut seperated = qb.separated(",");

		seperated.push_bind(common::database::Ulid(Ulid::new()));
		seperated.push_bind(access_token.organization_id);
		seperated.push_bind(transcoding_config.map(|t| t.id));
		seperated.push_bind(recording_config.map(|r| r.id));
		seperated.push_bind(Visibility::from(visibility));
		seperated.push_bind(create_stream_key());
		seperated.push_bind(sqlx::types::Json(self.tags.clone().unwrap_or_default().tags));

		qb.push(") RETURNING *");

		Ok(qb)
	}
}

impl QbResponse for RoomCreateResponse {
	type Request = RoomCreateRequest;

	fn from_query_object(query_object: Vec<<Self::Request as QbRequest>::QueryObject>) -> tonic::Result<Self> {
		if query_object.is_empty() {
			return Err(Status::internal(format!(
				"failed to create {}, no rows returned",
				<Self::Request as TonicRequest>::Table::FRIENDLY_NAME
			)));
		}

		let query_object = query_object.into_iter().next().unwrap();

		Ok(Self {
			stream_key: query_object.stream_key.clone(),
			room: Some(query_object.into_proto()),
		})
	}
}
