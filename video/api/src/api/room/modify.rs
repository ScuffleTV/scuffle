use std::sync::Arc;

use pb::ext::UlidExt;
use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::Resource;
use pb::scuffle::video::v1::{RoomModifyRequest, RoomModifyResponse};
use tonic::Status;
use video_common::database::{AccessToken, DatabaseTable};

use crate::api::utils::tags::validate_tags;
use crate::api::utils::{impl_request_scopes, QbRequest, QbResponse, TonicRequest};
use crate::global::ApiGlobal;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	RoomModifyRequest,
	video_common::database::Room,
	(Resource::Room, Permission::Modify),
	RateLimitResource::RoomModify
);

#[async_trait::async_trait]
impl QbRequest for RoomModifyRequest {
	type QueryObject = Self::Table;

	async fn build_query<G: ApiGlobal>(
		&self,
		global: &Arc<G>,
		access_token: &AccessToken,
	) -> tonic::Result<sqlx::QueryBuilder<'_, sqlx::Postgres>> {
		validate_tags(self.tags.as_ref())?;

		let mut qb = sqlx::query_builder::QueryBuilder::default();

		qb.push("UPDATE ").push(Self::Table::NAME).push(" SET ");

		let mut seperated = qb.separated(", ");

		if let Some(transcoding_config_id) = &self.transcoding_config_id {
			let transcoding_config_id = transcoding_config_id.to_ulid();
			if transcoding_config_id.is_nil() {
				seperated.push("transcoding_config_id = NULL");
			} else {
				let _: i32 = sqlx::query_scalar("SELECT 1 FROM transcoding_configs WHERE id = $1 AND organization_id = $2")
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
					.push_bind(common::database::Ulid(transcoding_config_id));
			}
		}

		if let Some(recording_config_id) = &self.recording_config_id {
			let recording_config_id = recording_config_id.to_ulid();
			if recording_config_id.is_nil() {
				seperated.push("recording_config_id = NULL");
			} else {
				let _: i32 = sqlx::query_scalar("SELECT 1 FROM recording_configs WHERE id = $1 AND organization_id = $2")
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
					.push_bind(common::database::Ulid(recording_config_id));
			}
		}

		if let Some(private) = &self.private {
			seperated.push("private = ").push_bind_unseparated(private);
		}

		if let Some(tags) = &self.tags {
			seperated.push("tags = ").push_bind_unseparated(sqlx::types::Json(&tags.tags));
		}

		seperated.push("updated_at = NOW()");

		qb.push(" WHERE id = ").push_bind(common::database::Ulid(self.id.to_ulid()));
		qb.push(" AND organization_id = ").push_bind(access_token.organization_id);
		qb.push(" RETURNING *");

		Ok(qb)
	}
}

impl QbResponse for RoomModifyResponse {
	type Request = RoomModifyRequest;

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
			room: Some(query_object.into_iter().next().unwrap().into_proto()),
		})
	}
}
