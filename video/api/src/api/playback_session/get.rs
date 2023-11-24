use std::sync::Arc;

use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::{playback_session_target, Resource};
use pb::scuffle::video::v1::{PlaybackSessionGetRequest, PlaybackSessionGetResponse};
use video_common::database::{AccessToken, DatabaseTable};

use crate::api::utils::{get, impl_request_scopes, QbRequest, QbResponse};
use crate::global::ApiGlobal;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	PlaybackSessionGetRequest,
	video_common::database::PlaybackSession,
	(Resource::PlaybackSession, Permission::Read),
	RateLimitResource::PlaybackSessionGet
);

#[async_trait::async_trait]
impl QbRequest for PlaybackSessionGetRequest {
	type QueryObject = Self::Table;

	async fn build_query<G: ApiGlobal>(
		&self,
		_: &Arc<G>,
		access_token: &AccessToken,
	) -> tonic::Result<sqlx::QueryBuilder<'_, sqlx::Postgres>> {
		let mut qb = sqlx::query_builder::QueryBuilder::default();
		qb.push("SELECT * FROM ").push(Self::Table::NAME).push(" WHERE ");
		let mut seperated = qb.separated(" AND ");

		get::organization_id(&mut seperated, access_token.organization_id);
		get::ids(&mut seperated, &self.ids);

		if let Some(user_id) = self.user_id.as_ref() {
			seperated.push("user_id = ");
			seperated.push_bind_unseparated(user_id);
		}

		if let Some(playback_key_pair_id) = self.playback_key_pair_id {
			seperated.push("playback_key_pair_id = ");
			seperated.push_bind_unseparated(common::database::Ulid(playback_key_pair_id.into_ulid()));
		} else if let Some(authorized) = self.authorized {
			if authorized {
				seperated.push("playback_key_pair_id IS NOT NULL");
			} else {
				seperated.push("playback_key_pair_id IS NULL");
			}
		}

		if let Some(ip_address) = self.ip_address.as_ref() {
			let ip = ip_address
				.parse::<std::net::IpAddr>()
				.map_err(|_| tonic::Status::invalid_argument(format!("invalid ip address: {}", ip_address)))?;

			seperated.push("ip_address = ");
			seperated.push_bind_unseparated(ip);
		}

		if let Some(target) = self.target {
			match target.target {
				Some(playback_session_target::Target::RecordingId(recording_id)) => {
					seperated.push("recording_id = ");
					seperated.push_bind_unseparated(common::database::Ulid(recording_id.into_ulid()));
				}
				Some(playback_session_target::Target::RoomId(room_id)) => {
					seperated.push("room_id = ");
					seperated.push_bind_unseparated(common::database::Ulid(room_id.into_ulid()));
				}
				None => {}
			}
		}

		if let Some(tags) = self.search_options.as_ref().and_then(|o| o.tags.as_ref()) {
			if !tags.tags.is_empty() {
				return Err(tonic::Status::invalid_argument("tags are not supported by playback sessions"));
			}
		}

		get::search_options(&mut seperated, self.search_options.as_ref())?;

		Ok(qb)
	}
}

impl QbResponse for PlaybackSessionGetResponse {
	type Request = PlaybackSessionGetRequest;

	fn from_query_object(query_objects: Vec<<Self::Request as QbRequest>::QueryObject>) -> tonic::Result<Self> {
		Ok(Self {
			sessions: query_objects
				.into_iter()
				.map(video_common::database::PlaybackSession::into_proto)
				.collect(),
		})
	}
}
