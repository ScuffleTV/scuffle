use std::sync::Arc;

use pb::ext::UlidExt;
use pb::scuffle::video::v1::events_fetch_request::Target;
use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::{event, Resource};
use pb::scuffle::video::v1::{RoomModifyRequest, RoomModifyResponse};
use tonic::Status;
use video_common::database::{AccessToken, DatabaseTable, Visibility};

use crate::api::errors::MODIFY_NO_FIELDS;
use crate::api::utils::tags::validate_tags;
use crate::api::utils::{events, impl_request_scopes, ApiRequest, TonicRequest};
use crate::global::ApiGlobal;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	RoomModifyRequest,
	video_common::database::Room,
	(Resource::Room, Permission::Modify),
	RateLimitResource::RoomModify
);

pub fn validate(req: &RoomModifyRequest) -> tonic::Result<()> {
	validate_tags(req.tags.as_ref())
}

pub async fn build_query<'a, G: ApiGlobal>(
	req: &'a RoomModifyRequest,
	global: &'a Arc<G>,
	access_token: &AccessToken,
) -> tonic::Result<sqlx::QueryBuilder<'a, sqlx::Postgres>> {
	let mut qb = sqlx::query_builder::QueryBuilder::default();

	qb.push("UPDATE ")
		.push(<RoomModifyRequest as TonicRequest>::Table::NAME)
		.push(" SET ");

	let mut seperated = qb.separated(",");

	if let Some(transcoding_config_id) = &req.transcoding_config_id {
		let transcoding_config_id = transcoding_config_id.into_ulid();
		if transcoding_config_id.is_nil() {
			seperated.push("transcoding_config_id = NULL");
		} else {
			let _: i64 = sqlx::query_scalar("SELECT 1 FROM transcoding_configs WHERE id = $1 AND organization_id = $2")
				.bind(common::database::Ulid(transcoding_config_id))
				.bind(access_token.organization_id)
				.fetch_optional(global.db().as_ref())
				.await
				.map_err(|err| {
					tracing::error!(err = %err, "failed to fetch transcoding config");
					Status::internal("failed to fetch transcoding config")
				})?
				.ok_or_else(|| Status::not_found("transcoding config not found"))?;

			seperated
				.push("transcoding_config_id = ")
				.push_bind_unseparated(common::database::Ulid(transcoding_config_id));
		}
	}

	if let Some(recording_config_id) = &req.recording_config_id {
		let recording_config_id = recording_config_id.into_ulid();
		if recording_config_id.is_nil() {
			seperated.push("recording_config_id = NULL");
		} else {
			let _: i64 = sqlx::query_scalar("SELECT 1 FROM recording_configs WHERE id = $1 AND organization_id = $2")
				.bind(common::database::Ulid(recording_config_id))
				.bind(access_token.organization_id)
				.fetch_optional(global.db().as_ref())
				.await
				.map_err(|err| {
					tracing::error!(err = %err, "failed to fetch recording config");
					Status::internal("failed to fetch recording config")
				})?
				.ok_or_else(|| Status::not_found("recording config not found"))?;

			seperated
				.push("recording_config_id = ")
				.push_bind_unseparated(common::database::Ulid(recording_config_id));
		}
	}

	if let Some(visibility) = req.visibility {
		let visibility = pb::scuffle::video::v1::types::Visibility::try_from(visibility)
			.map_err(|_| Status::invalid_argument("invalid visibility value"))?;

		seperated
			.push("visibility = ")
			.push_bind_unseparated(Visibility::from(visibility));
	}

	if let Some(tags) = &req.tags {
		seperated.push("tags = ").push_bind_unseparated(sqlx::types::Json(&tags.tags));
	}

	if req.tags.is_none()
		&& req.transcoding_config_id.is_none()
		&& req.recording_config_id.is_none()
		&& req.visibility.is_none()
	{
		return Err(Status::invalid_argument(MODIFY_NO_FIELDS));
	}

	seperated.push("updated_at = NOW()");

	qb.push(" WHERE id = ").push_bind(common::database::Ulid(req.id.into_ulid()));
	qb.push(" AND organization_id = ").push_bind(access_token.organization_id);
	qb.push(" RETURNING *");

	Ok(qb)
}

#[async_trait::async_trait]
impl ApiRequest<RoomModifyResponse> for tonic::Request<RoomModifyRequest> {
	async fn process<G: ApiGlobal>(
		&self,
		global: &Arc<G>,
		access_token: &AccessToken,
	) -> tonic::Result<tonic::Response<RoomModifyResponse>> {
		let req = self.get_ref();

		validate(req)?;

		let mut query = build_query(req, global, access_token).await?;

		let result: Option<video_common::database::Room> = query
			.build_query_as()
			.fetch_optional(global.db().as_ref())
			.await
			.map_err(|err| {
				tracing::error!(err = %err, "failed to modify {}", <RoomModifyRequest as TonicRequest>::Table::FRIENDLY_NAME);
				tonic::Status::internal(format!(
					"failed to modify {}",
					<RoomModifyRequest as TonicRequest>::Table::FRIENDLY_NAME
				))
			})?;

		let Some(result) = result else {
			return Err(tonic::Status::not_found(format!(
				"{} not found",
				<RoomModifyRequest as TonicRequest>::Table::FRIENDLY_NAME
			)));
		};

		events::emit(
			global,
			access_token.organization_id.0,
			Target::Room,
			event::Event::Room(event::Room {
				room_id: Some(result.id.0.into()),
				event: Some(event::room::Event::Modified(event::room::Modified {})),
			}),
		)
		.await;

		Ok(tonic::Response::new(RoomModifyResponse {
			room: Some(result.into_proto()),
		}))
	}
}
