use std::collections::HashSet;
use std::sync::{Arc, Weak};

use chrono::TimeZone;
use pb::ext::UlidExt;
use pb::scuffle::video::v1::access_token_server::{
	AccessToken as AccessTokenServiceTrait, AccessTokenServer as AccessTokenService,
};
use pb::scuffle::video::v1::types::access_token_scope::{Permission, Resource};
use pb::scuffle::video::v1::types::Tags;
use pb::scuffle::video::v1::{
	AccessTokenCreateRequest, AccessTokenCreateResponse, AccessTokenDeleteRequest, AccessTokenDeleteResponse,
	AccessTokenGetRequest, AccessTokenGetResponse, AccessTokenTagRequest, AccessTokenTagResponse, AccessTokenUntagRequest,
	AccessTokenUntagResponse,
};
use tonic::{async_trait, Request, Response, Status};
use ulid::Ulid;
use video_common::database::AccessToken;

use super::utils::{
	add_tag_query, get_global, remove_tag_query, validate_auth_request, validate_tags, AccessTokenExt, RequiredScope,
};
use crate::global::ApiGlobal;

mod utils;

type Result<T> = std::result::Result<T, Status>;

pub struct AccessTokenServer<G: ApiGlobal> {
	global: Weak<G>,
}

impl<G: ApiGlobal> AccessTokenServer<G> {
	pub fn new(global: &Arc<G>) -> AccessTokenService<Self> {
		AccessTokenService::new(Self {
			global: Arc::downgrade(global),
		})
	}
}

#[async_trait]
impl<G: ApiGlobal> AccessTokenServiceTrait for AccessTokenServer<G> {
	async fn get(&self, request: Request<AccessTokenGetRequest>) -> Result<Response<AccessTokenGetResponse>> {
		let global = get_global(&self.global)?;

		let access_token = validate_auth_request(&global, &request, (Resource::AccessToken, Permission::Read)).await?;

		let mut access_tokens = utils::get_access_tokens(&access_token, request.get_ref())?;

		let results: Vec<AccessToken> =
			access_tokens
				.build_query_as()
				.fetch_all(global.db().as_ref())
				.await
				.map_err(|err| {
					tracing::error!("failed to fetch access tokens: {}", err);
					Status::internal("failed to fetch access tokens")
				})?;

		Ok(Response::new(AccessTokenGetResponse {
			access_tokens: results.into_iter().map(|access_token| access_token.to_proto()).collect(),
		}))
	}

	async fn create(&self, request: Request<AccessTokenCreateRequest>) -> Result<Response<AccessTokenCreateResponse>> {
		let global = get_global(&self.global)?;

		let access_token = validate_auth_request(&global, &request, (Resource::AccessToken, Permission::Create)).await?;

		let permissions = RequiredScope::from(request.get_ref().scopes.clone());

		access_token.has_scope(permissions)?;

		validate_tags(request.get_ref().tags.as_ref())?;

		let access_token = AccessToken {
			id: common::database::Ulid(Ulid::new()),
			secret_key: common::database::Ulid(Ulid::new()),
			organization_id: access_token.organization_id,
			tags: request.get_ref().tags.clone().unwrap_or_default().tags.into(),
			last_active_at: None,
			updated_at: chrono::Utc::now(),
			expires_at: request
				.get_ref()
				.expires_at
				.and_then(|s| chrono::Utc.timestamp_millis_opt(s).single()),
			scopes: request
				.get_ref()
				.scopes
				.clone()
				.into_iter()
				.map(|scope| scope.into())
				.collect(),
		};

		sqlx::query(
			r#"
            INSERT INTO access_tokens (
                id,
                secret_key,
                organization_id,
                tags,
                last_active_at,
                updated_at,
                expires_at,
                scopes
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
		)
		.bind(access_token.id)
		.bind(access_token.secret_key)
		.bind(access_token.organization_id)
		.bind(&access_token.tags)
		.bind(access_token.last_active_at)
		.bind(access_token.updated_at)
		.bind(access_token.expires_at)
		.bind(&access_token.scopes)
		.execute(global.db().as_ref())
		.await
		.map_err(|err| {
			tracing::error!("failed to create access token: {}", err);
			Status::internal("failed to create access token")
		})?;

		Ok(Response::new(AccessTokenCreateResponse {
			access_token: Some(access_token.to_proto()),
		}))
	}

	async fn delete(&self, request: Request<AccessTokenDeleteRequest>) -> Result<Response<AccessTokenDeleteResponse>> {
		let global = get_global(&self.global)?;

		let access_token = validate_auth_request(&global, &request, (Resource::AccessToken, Permission::Delete)).await?;

		if request.get_ref().ids.len() > 100 {
			return Err(Status::invalid_argument("too many ids provided for delete: max 100"));
		}

		if request.get_ref().ids.is_empty() {
			return Err(Status::invalid_argument("no ids provided for delete, minimum 1"));
		}

		let ids = request.get_ref().ids.iter().map(|id| id.to_ulid()).collect::<HashSet<_>>();

		if ids.contains(&access_token.id.0) {
			return Err(Status::invalid_argument("cannot delete your own access token"));
		}

		let tokens_to_delete = global
			.access_token_loader()
			.load_many(ids.into_iter())
			.await
			.map_err(|_| Status::internal("failed to load access tokens for delete"))?
			.into_values()
			.filter(|token| token.organization_id == access_token.organization_id)
			.collect::<Vec<_>>();

		// We need to make sure we have all the permissions of every token we try to
		// delete.
		for delete_token in &tokens_to_delete {
			access_token
				.has_scope(delete_token.scopes.iter().map(|scope| scope.0.clone()).collect::<Vec<_>>())
				.map_err(|_| Status::permission_denied("cannot delete access token with more permissions than you"))?;
		}

		sqlx::query(
			r#"
            DELETE FROM access_tokens
            WHERE id = ANY($1)
            AND organization_id = $2
            "#,
		)
		.bind(&tokens_to_delete.iter().map(|token| token.id).collect::<Vec<_>>())
		.bind(access_token.organization_id)
		.execute(global.db().as_ref())
		.await
		.map_err(|err| {
			tracing::error!("failed to delete access token: {}", err);
			Status::internal("failed to delete access token")
		})?;

		Ok(Response::new(AccessTokenDeleteResponse {
			ids: tokens_to_delete.into_iter().map(|token| token.id.0.into()).collect(),
		}))
	}

	async fn tag(&self, request: Request<AccessTokenTagRequest>) -> Result<Response<AccessTokenTagResponse>> {
		let global = get_global(&self.global)?;

		let access_token = validate_auth_request(&global, &request, (Resource::AccessToken, Permission::Modify)).await?;

		let Some(tags) = request.get_ref().tags.as_ref() else {
			return Err(Status::invalid_argument("tags must be provided"));
		};

		if tags.tags.is_empty() {
			return Err(Status::invalid_argument("tags must not be empty"));
		}

		validate_tags(Some(tags))?;

		let id = request.get_ref().id.to_ulid();

		let updated_tags = add_tag_query(
			&global,
			"access_tokens",
			&tags.tags,
			id,
			Some(access_token.organization_id.into()),
		)
		.await?
		.ok_or_else(|| Status::not_found("access token not found"))?;

		Ok(Response::new(AccessTokenTagResponse {
			tags: Some(Tags { tags: updated_tags }),
		}))
	}

	async fn untag(&self, request: Request<AccessTokenUntagRequest>) -> Result<Response<AccessTokenUntagResponse>> {
		let global = get_global(&self.global)?;

		let access_token = validate_auth_request(&global, &request, (Resource::AccessToken, Permission::Modify)).await?;

		if request.get_ref().tags.is_empty() {
			return Err(Status::invalid_argument("tags must not be empty"));
		}

		let id = request.get_ref().id.to_ulid();

		let updated_tags = remove_tag_query(
			&global,
			"access_tokens",
			&request.get_ref().tags,
			id,
			Some(access_token.organization_id.into()),
		)
		.await?
		.ok_or_else(|| Status::not_found("access token not found"))?;

		Ok(Response::new(AccessTokenUntagResponse {
			tags: Some(Tags { tags: updated_tags }),
		}))
	}
}
