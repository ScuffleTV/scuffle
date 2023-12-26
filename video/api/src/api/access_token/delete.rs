use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use pb::scuffle::video::v1::events_fetch_request::Target;
use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::{event, FailedResource, Resource};
use pb::scuffle::video::v1::{AccessTokenDeleteRequest, AccessTokenDeleteResponse};
use tonic::Status;
use video_common::database::{AccessToken, DatabaseTable};

use crate::api::utils::{events, impl_request_scopes, AccessTokenExt, ApiRequest, TonicRequest};
use crate::global::ApiGlobal;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	AccessTokenDeleteRequest,
	video_common::database::AccessToken,
	(Resource::AccessToken, Permission::Delete),
	RateLimitResource::AccessTokenDelete
);

impl ApiRequest<AccessTokenDeleteResponse> for tonic::Request<AccessTokenDeleteRequest> {
	async fn process<G: ApiGlobal>(
		&self,
		global: &Arc<G>,
		access_token: &AccessToken,
	) -> tonic::Result<tonic::Response<AccessTokenDeleteResponse>> {
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

		let tokens_to_delete = global
			.access_token_loader()
			.load_many(ids_to_delete.iter().copied().map(|id| (access_token.organization_id.0, id)))
			.await
			.map_err(|_| Status::internal("failed to load access tokens for delete"))?
			.into_values()
			.filter(|token| token.organization_id == access_token.organization_id)
			.collect::<Vec<_>>();

		let mut failed_tokens = tokens_to_delete
			.iter()
			.filter(|delete_token| {
				access_token
					.has_scope(
						&delete_token
							.scopes
							.iter()
							.map(|scope| scope.0.clone())
							.collect::<Vec<_>>()
							.into(),
					)
					.is_err()
			})
			.map(|token| (token.id.0, "cannot delete access token with more permissions then requester"))
			.collect::<HashMap<_, _>>();

		if ids_to_delete.remove(&access_token.id.0) {
			failed_tokens.insert(access_token.id.0, "cannot delete own access token");
		}

		let deleted_ids = if !ids_to_delete.is_empty() {
			let mut qb = sqlx::query_builder::QueryBuilder::default();

			qb.push("DELETE FROM ")
				.push(<AccessTokenDeleteRequest as TonicRequest>::Table::NAME)
				.push(" WHERE id = ANY(")
				.push_bind(ids_to_delete.iter().copied().map(common::database::Ulid).collect::<Vec<_>>())
				.push(") AND organization_id = ")
				.push_bind(access_token.organization_id)
				.push(" RETURNING id");

			let deleted_ids: Vec<common::database::Ulid> =
				qb.build_query_scalar().fetch_all(global.db().as_ref()).await.map_err(|err| {
					tracing::error!(err = %err, "failed to delete {}", <AccessTokenDeleteRequest as TonicRequest>::Table::FRIENDLY_NAME);
					Status::internal(format!(
						"failed to delete {}",
						<AccessTokenDeleteRequest as TonicRequest>::Table::FRIENDLY_NAME
					))
				})?;

			deleted_ids.iter().for_each(|id| {
				ids_to_delete.remove(&id.0);
			});

			deleted_ids
		} else {
			Default::default()
		};

		for id in deleted_ids.iter().copied() {
			events::emit(
				global,
				access_token.organization_id.0,
				Target::AccessToken,
				event::Event::AccessToken(event::AccessToken {
					access_token_id: Some(id.0.into()),
					event: Some(event::access_token::Event::Deleted(event::access_token::Deleted {})),
				}),
			)
			.await;
		}

		ids_to_delete.into_iter().for_each(|id| {
			failed_tokens.insert(id, "access token not found");
		});

		Ok(tonic::Response::new(AccessTokenDeleteResponse {
			ids: deleted_ids.into_iter().map(|id| id.0.into()).collect(),
			failed_deletes: failed_tokens
				.into_iter()
				.map(|(id, reason)| FailedResource {
					id: Some(id.into()),
					reason: reason.to_string(),
				})
				.collect(),
		}))
	}
}
