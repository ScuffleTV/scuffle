use std::collections::HashSet;
use std::sync::Arc;

use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::{FailedResource, Resource};
use pb::scuffle::video::v1::{PlaybackKeyPairDeleteRequest, PlaybackKeyPairDeleteResponse};
use tonic::Status;
use video_common::database::{AccessToken, DatabaseTable};

use crate::api::utils::{impl_request_scopes, ApiRequest, TonicRequest};
use crate::global::ApiGlobal;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	PlaybackKeyPairDeleteRequest,
	video_common::database::PlaybackKeyPair,
	(Resource::PlaybackKeyPair, Permission::Delete),
	RateLimitResource::PlaybackKeyPairDelete
);

#[async_trait::async_trait]
impl ApiRequest<PlaybackKeyPairDeleteResponse> for tonic::Request<PlaybackKeyPairDeleteRequest> {
	async fn process<G: ApiGlobal>(
		&self,
		global: &Arc<G>,
		access_token: &AccessToken,
	) -> tonic::Result<tonic::Response<PlaybackKeyPairDeleteResponse>> {
		let mut qb = sqlx::query_builder::QueryBuilder::default();

		let req = self.get_ref();

		if req.ids.len() > 100 {
			return Err(tonic::Status::invalid_argument("too many ids provided for delete: max 100"));
		}

		if req.ids.is_empty() {
			return Err(tonic::Status::invalid_argument("no ids provided for delete"));
		}

		let mut ids_to_delete = req
			.ids
			.iter()
			.copied()
			.map(pb::scuffle::types::Ulid::into_ulid)
			.collect::<HashSet<_>>();

		qb.push("DELETE FROM ")
			.push(<PlaybackKeyPairDeleteRequest as TonicRequest>::Table::NAME)
			.push(" WHERE id = ANY(")
			.push_bind(ids_to_delete.iter().copied().map(common::database::Ulid).collect::<Vec<_>>())
			.push(") AND organization_id = ")
			.push_bind(access_token.organization_id)
			.push(" RETURNING id");

		let deleted_ids: Vec<common::database::Ulid> =
			qb.build_query_scalar().fetch_all(global.db().as_ref()).await.map_err(|err| {
				tracing::error!(err = %err, "failed to delete {}", <PlaybackKeyPairDeleteRequest as TonicRequest>::Table::FRIENDLY_NAME);
				Status::internal(format!(
					"failed to delete {}",
					<PlaybackKeyPairDeleteRequest as TonicRequest>::Table::FRIENDLY_NAME
				))
			})?;

		deleted_ids.iter().for_each(|id| {
			ids_to_delete.remove(&id.0);
		});

		Ok(tonic::Response::new(PlaybackKeyPairDeleteResponse {
			ids: deleted_ids.into_iter().map(|id| id.0.into()).collect(),
			failed_deletes: ids_to_delete
				.into_iter()
				.map(|id| FailedResource {
					id: Some(id.into()),
					reason: "playback key pair not found".into(),
				})
				.collect(),
		}))
	}
}
