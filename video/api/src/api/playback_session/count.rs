use std::sync::Arc;

use pb::ext::UlidExt;
use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::{playback_session_target, Resource};
use pb::scuffle::video::v1::{playback_session_count_request, PlaybackSessionCountRequest, PlaybackSessionCountResponse};
use video_common::database::{AccessToken, DatabaseTable};

use crate::api::utils::{impl_request_scopes, QbRequest, QbResponse, TonicRequest};
use crate::global::ApiGlobal;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	PlaybackSessionCountRequest,
	video_common::database::PlaybackSession,
	(Resource::PlaybackSession, Permission::Read),
	RateLimitResource::PlaybackSessionCount
);

#[derive(sqlx::FromRow)]
pub struct PlaybackSessionCountQueryResp {
	total_count: i64,
	deduped: i64,
}

#[async_trait::async_trait]
impl QbRequest for PlaybackSessionCountRequest {
	type QueryObject = PlaybackSessionCountQueryResp;

	async fn build_query<G: ApiGlobal>(
		&self,
		_: &Arc<G>,
		access_token: &AccessToken,
	) -> tonic::Result<sqlx::query_builder::QueryBuilder<'_, sqlx::Postgres>> {
		let mut qb = sqlx::query_builder::QueryBuilder::default();

		let filter = self
			.filter
			.as_ref()
			.ok_or_else(|| tonic::Status::invalid_argument("filter is required"))?;

		qb.push("SELECT COUNT(*) AS total_count, COUNT(DISTINCT ");

		match filter {
			playback_session_count_request::Filter::UserId(user_id) => {
				qb.push("recording_id, room_id) AS deduped FROM playback_sessions WHERE user_id = ")
					.push_bind(user_id)
					.push(" AND organization_id = ")
					.push_bind(access_token.organization_id);
			}
			playback_session_count_request::Filter::Target(target) => {
				let target = target
					.target
					.as_ref()
					.ok_or_else(|| tonic::Status::invalid_argument("filter is required"))?;

				qb.push("user_id, ip_address) AS deduped FROM playback_sessions WHERE organization_id = ")
					.push_bind(access_token.organization_id);
				qb.push(" AND ");

				match target {
					playback_session_target::Target::RecordingId(recording_id) => {
						qb.push("recording_id = ")
							.push_bind(common::database::Ulid(recording_id.to_ulid()));
					}
					playback_session_target::Target::RoomId(room_id) => {
						qb.push("room_id = ").push_bind(common::database::Ulid(room_id.to_ulid()));
					}
				}
			}
		}

		Ok(qb)
	}
}

impl QbResponse for PlaybackSessionCountResponse {
	type Request = PlaybackSessionCountRequest;

	fn from_query_object(query_object: Vec<<Self::Request as QbRequest>::QueryObject>) -> tonic::Result<Self> {
		if query_object.is_empty() {
			return Ok(Self {
				count: 0,
				deduplicated_count: 0,
			});
		}

		if query_object.len() > 1 {
			return Err(tonic::Status::internal(format!(
				"failed to query {}, {} rows returned",
				<Self::Request as TonicRequest>::Table::FRIENDLY_NAME,
				query_object.len(),
			)));
		}

		let query_object = query_object.into_iter().next().unwrap();

		Ok(Self {
			count: query_object.total_count as u64,
			deduplicated_count: query_object.deduped as u64,
		})
	}
}
