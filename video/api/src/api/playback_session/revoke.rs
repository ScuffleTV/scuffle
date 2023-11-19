use std::collections::HashSet;
use std::sync::Arc;

use pb::ext::UlidExt;
use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::{playback_session_target, Resource};
use pb::scuffle::video::v1::{PlaybackSessionRevokeRequest, PlaybackSessionRevokeResponse};
use video_common::database::AccessToken;

use crate::api::utils::{impl_request_scopes, QbRequest, QbResponse};
use crate::global::ApiGlobal;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	PlaybackSessionRevokeRequest,
	video_common::database::PlaybackSession,
	(Resource::PlaybackSession, Permission::Delete),
	RateLimitResource::PlaybackSessionRevoke
);

#[derive(sqlx::FromRow)]
pub struct PlaybackSessionRevokeQueryResp {
	deleted_count: i64,
}

#[async_trait::async_trait]
impl QbRequest for PlaybackSessionRevokeRequest {
	type QueryObject = PlaybackSessionRevokeQueryResp;

	async fn build_query<G: ApiGlobal>(
		&self,
		_: &Arc<G>,
		access_token: &AccessToken,
	) -> tonic::Result<sqlx::query_builder::QueryBuilder<'_, sqlx::Postgres>> {
		let mut qb = sqlx::query_builder::QueryBuilder::default();

		if self.ids.len() > 100 {
			return Err(tonic::Status::invalid_argument("too many ids provided for revoke: max 100"));
		}

		qb.push("DELETE FROM playback_sessions WHERE ");

		let mut seperated = qb.separated(" AND ");

		seperated
			.push("organization_id = ")
			.push_bind_unseparated(access_token.organization_id);

		if !self.ids.is_empty() {
			let ids = self.ids.iter().map(pb::ext::UlidExt::to_ulid).collect::<HashSet<_>>();

			seperated
				.push("id = ANY(")
				.push_bind_unseparated(ids.into_iter().map(common::database::Ulid).collect::<Vec<_>>())
				.push_bind_unseparated(")");
		}

		if let Some(user_id) = &self.user_id {
			seperated.push("user_id = ").push_bind_unseparated(user_id);
		}

		if let Some(target) = &self.target {
			match &target.target {
				Some(playback_session_target::Target::RecordingId(recording_id)) => {
					seperated
						.push("recording_id = ")
						.push_bind_unseparated(common::database::Ulid(recording_id.to_ulid()));
				}
				Some(playback_session_target::Target::RoomId(room_id)) => {
					seperated
						.push("room_id = ")
						.push_bind_unseparated(common::database::Ulid(room_id.to_ulid()));
				}
				None => {}
			}
		}

		qb.push(" RETURNING COUNT(*) AS deleted_count");

		Ok(qb)
	}
}

impl QbResponse for PlaybackSessionRevokeResponse {
	type Request = PlaybackSessionRevokeRequest;

	fn from_query_object(query_object: Vec<<Self::Request as QbRequest>::QueryObject>) -> tonic::Result<Self> {
		if query_object.is_empty() {
			return Err(tonic::Status::not_found("no playback sessions found"));
		}

		let deleted_count = query_object[0].deleted_count;

		Ok(Self {
			revoked: deleted_count as u64,
		})
	}
}
