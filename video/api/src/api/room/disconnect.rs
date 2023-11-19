use std::collections::{HashSet, HashMap};
use std::sync::Arc;

use bytes::Bytes;
use pb::ext::UlidExt;
use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::Resource;
use pb::scuffle::video::v1::{RoomDisconnectRequest, RoomDisconnectResponse};
use video_common::database::{AccessToken, RoomStatus};
use video_common::keys;

use crate::api::utils::{impl_request_scopes, ApiRequest};
use crate::global::ApiGlobal;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	RoomDisconnectRequest,
	video_common::database::Room,
	(Resource::Room, Permission::Modify),
	RateLimitResource::RoomDisconnect
);

#[async_trait::async_trait]
impl ApiRequest<RoomDisconnectResponse> for tonic::Request<RoomDisconnectRequest> {
	async fn process<G: ApiGlobal>(
		&self,
		global: &Arc<G>,
		access_token: &AccessToken,
	) -> tonic::Result<tonic::Response<RoomDisconnectResponse>> {
		let req = self.get_ref();

		if req.ids.is_empty() {
			return Err(tonic::Status::invalid_argument("cannot disconnect 0 rooms"));
		}

		if req.ids.len() > 100 {
			return Err(tonic::Status::invalid_argument("cannot disconnect more than 100 rooms in a single request"));
		}

		let ids = req
			.ids
			.iter()
			.map(|id| id.to_ulid())
			.collect::<HashSet<_>>();

		let rooms = global.room_loader().load_many(ids.iter().copied()).await.map_err(|_| {
			tonic::Status::internal("failed to load rooms")
		})?;

		let mut failed_rooms = HashMap::new();

		let mut disconnected_ids = Vec::new();

		for id in ids {
			if let Some(room) = rooms.get(&id) {
				if room.organization_id != access_token.organization_id {
					failed_rooms.insert(id, "room does not exist");
				} else if room.status == RoomStatus::Offline || room.active_ingest_connection_id.is_none() {
					failed_rooms.insert(id, "room is already offline");
				} else if let Err(err) = global.nats().publish(keys::ingest_disconnect(id), Bytes::new()).await {
					tracing::error!(err = %err, "failed to publish ingest disconnect");
					failed_rooms.insert(id, "failed to publish ingest disconnect");
				} else {
					disconnected_ids.push(id);
				}
			} else {
				failed_rooms.insert(id, "room does not exist");
			}
		}

		Ok(tonic::Response::new(RoomDisconnectResponse {
			ids: rooms.into_keys().map(|id| id.into()).collect(),
			failed_disconnects: failed_rooms.into_iter().map(|(id, reason)| pb::scuffle::video::v1::types::FailedResource {
				id: Some(id.into()),
				reason: reason.to_string(),
			}).collect(),
		}))
	}
}
