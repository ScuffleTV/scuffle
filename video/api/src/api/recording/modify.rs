use std::sync::Arc;

use pb::ext::UlidExt;
use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::Resource;
use pb::scuffle::video::v1::{RecordingModifyRequest, RecordingModifyResponse};
use tonic::Status;
use video_common::database::{AccessToken, DatabaseTable};

use crate::api::utils::tags::validate_tags;
use crate::api::utils::{impl_request_scopes, ApiRequest, TonicRequest};
use crate::global::ApiGlobal;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	RecordingModifyRequest,
	video_common::database::Recording,
	(Resource::Recording, Permission::Modify),
	RateLimitResource::RecordingModify
);

#[async_trait::async_trait]
impl ApiRequest<RecordingModifyResponse> for tonic::Request<RecordingModifyRequest> {
	async fn process<G: ApiGlobal>(
		&self,
		global: &Arc<G>,
		access_token: &AccessToken,
	) -> tonic::Result<tonic::Response<RecordingModifyResponse>> {
		let req = self.get_ref();

		validate_tags(req.tags.as_ref())?;

		let mut qb = sqlx::query_builder::QueryBuilder::default();

		qb.push("UPDATE ")
			.push(<RecordingModifyRequest as TonicRequest>::Table::NAME)
			.push(" SET ");

		let mut seperated = qb.separated(", ");

		if let Some(room_id) = &req.room_id {
			sqlx::query("SELECT id FROM rooms WHERE id = $1 AND organization_id = $2")
				.bind(room_id.to_uuid())
				.fetch_optional(global.db().as_ref())
				.await
				.map_err(|err| {
					tracing::error!(err = %err, "failed to query room");
					Status::internal("failed to query rooms")
				})?
				.ok_or_else(|| Status::not_found("room not found"))?;

			seperated.push("room_id = ").push_bind_unseparated(room_id.to_uuid());
		}

		if let Some(recording_config_id) = &req.recording_config_id {
			sqlx::query("SELECT id FROM recording_configs WHERE id = $1 AND organization_id = $2")
				.bind(recording_config_id.to_uuid())
				.fetch_optional(global.db().as_ref())
				.await
				.map_err(|err| {
					tracing::error!(err = %err, "failed to query recording config");
					Status::internal("failed to query recording configs")
				})?
				.ok_or_else(|| Status::not_found("recording config not found"))?;

			seperated
				.push("recording_config_id = ")
				.push_bind_unseparated(recording_config_id.to_uuid());
		}

		if let Some(public) = req.public {
			seperated.push("public = ").push_bind_unseparated(public);
		}

		if let Some(tags) = &req.tags {
			seperated.push("tags = ").push_bind_unseparated(sqlx::types::Json(&tags.tags));
		}

		seperated.push("updated_at = ").push_bind_unseparated(chrono::Utc::now());

		qb.push(" WHERE id = ").push_bind(req.id.to_uuid());
		qb.push(" AND organization_id = ").push_bind(access_token.organization_id);
		qb.push(" RETURNING *");

		let recording = qb
			.build_query_as::<<RecordingModifyRequest as TonicRequest>::Table>()
			.fetch_optional(global.db().as_ref())
			.await
			.map_err(|err| {
				tracing::error!(err = %err, "failed to update recording");
				Status::internal("failed to update recording")
			})?
			.ok_or_else(|| Status::not_found("recording not found"))?;

		let state = global
			.recording_state_loader()
			.load(recording.id.0)
			.await
			.map_err(|_| Status::internal("failed to load recording state"))?
			.unwrap_or_default();

		Ok(tonic::Response::new(RecordingModifyResponse {
			recording: Some(state.recording_to_proto(recording)),
		}))
	}
}
