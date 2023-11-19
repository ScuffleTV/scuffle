use std::collections::HashSet;
use std::sync::Arc;

use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::{FailedResource, Resource};
use pb::scuffle::video::v1::{room_reset_key_response, RoomResetKeyRequest, RoomResetKeyResponse};
use video_common::database::{AccessToken, DatabaseTable};

use super::utils::create_stream_key;
use crate::api::utils::{impl_request_scopes, ApiRequest, TonicRequest};
use crate::global::ApiGlobal;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	RoomResetKeyRequest,
	video_common::database::Room,
	(Resource::Room, Permission::Modify),
	RateLimitResource::RoomResetKey
);

#[async_trait::async_trait]
impl ApiRequest<RoomResetKeyResponse> for tonic::Request<RoomResetKeyRequest> {
	async fn process<G: ApiGlobal>(
		&self,
		global: &Arc<G>,
		access_token: &AccessToken,
	) -> tonic::Result<tonic::Response<RoomResetKeyResponse>> {
		let req = self.get_ref();

		if req.ids.len() > 100 {
			return Err(tonic::Status::invalid_argument("too many ids provided for delete: max 100"));
		}

		if req.ids.is_empty() {
			return Err(tonic::Status::invalid_argument("no ids provided for delete"));
		}

		let mut ids_to_reset = req.ids.iter().map(pb::ext::UlidExt::to_ulid).collect::<HashSet<_>>();

		let data = ids_to_reset
			.iter()
			.map(|id| (common::database::Ulid(*id), create_stream_key()));

		let mut qb = sqlx::query_builder::QueryBuilder::default();

		qb.push("UPDATE ")
			.push(<RoomResetKeyRequest as TonicRequest>::Table::NAME)
			.push(" r SET r.stream_key = v.stream_key FROM (")
			.push_values(data, |mut b, data| {
				b.push_bind(data.0).push_bind(data.1);
			})
			.push(") AS v (id, stream_key) WHERE r.id = v.id AND r.organization_id = ")
			.push_bind(access_token.organization_id)
			.push(" RETURNING r.id, r.stream_key");

		let rows: Vec<(common::database::Ulid, String)> =
			qb.build_query_scalar().fetch_all(global.db().as_ref()).await.map_err(|err| {
				tracing::error!(err = %err, "failed to reset room stream keys");
				tonic::Status::internal("failed to reset room stream keys")
			})?;

		let rooms = rows
			.into_iter()
			.map(|(id, key)| {
				ids_to_reset.remove(&id.0);

				room_reset_key_response::RoomKeyPair {
					id: Some(id.0.into()),
					key,
				}
			})
			.collect::<Vec<_>>();

		let failed_resets = ids_to_reset
			.into_iter()
			.map(|id| FailedResource {
				id: Some(id.into()),
				reason: "room not found".into(),
			})
			.collect::<Vec<_>>();

		Ok(tonic::Response::new(RoomResetKeyResponse { rooms, failed_resets }))
	}
}
