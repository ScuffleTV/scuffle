use std::sync::{Arc, Weak};

use pb::ext::UlidExt;
use pb::scuffle::video::v1::transcoding_config_server::{
	TranscodingConfig as TranscodingConfigTrait, TranscodingConfigServer as TranscodingConfigService,
};
use pb::scuffle::video::v1::types::access_token_scope::{Permission, Resource};
use pb::scuffle::video::v1::types::Tags;
use pb::scuffle::video::v1::{
	TranscodingConfigCreateRequest, TranscodingConfigCreateResponse, TranscodingConfigDeleteRequest,
	TranscodingConfigDeleteResponse, TranscodingConfigGetRequest, TranscodingConfigGetResponse,
	TranscodingConfigModifyRequest, TranscodingConfigModifyResponse, TranscodingConfigTagRequest,
	TranscodingConfigTagResponse, TranscodingConfigUntagRequest, TranscodingConfigUntagResponse,
};
use tonic::{async_trait, Request, Response, Status};

use super::utils::{add_tag_query, get_global, remove_tag_query, validate_auth_request, validate_tags};
use crate::global::ApiGlobal;

type Result<T> = std::result::Result<T, Status>;

pub struct TranscodingConfigServer<G: ApiGlobal> {
	global: Weak<G>,
}

impl<G: ApiGlobal> TranscodingConfigServer<G> {
	pub fn new(global: &Arc<G>) -> TranscodingConfigService<Self> {
		TranscodingConfigService::new(Self {
			global: Arc::downgrade(global),
		})
	}
}

#[async_trait]
impl<G: ApiGlobal> TranscodingConfigTrait for TranscodingConfigServer<G> {
	async fn get(&self, _request: Request<TranscodingConfigGetRequest>) -> Result<Response<TranscodingConfigGetResponse>> {
		todo!("TODO: implement TranscodingConfig::get")
	}

	async fn create(
		&self,
		_request: Request<TranscodingConfigCreateRequest>,
	) -> Result<Response<TranscodingConfigCreateResponse>> {
		todo!("TODO: implement TranscodingConfig::create")
	}

	async fn modify(
		&self,
		_request: Request<TranscodingConfigModifyRequest>,
	) -> Result<Response<TranscodingConfigModifyResponse>> {
		todo!("TODO: implement TranscodingConfig::modify")
	}

	async fn delete(
		&self,
		_request: Request<TranscodingConfigDeleteRequest>,
	) -> Result<Response<TranscodingConfigDeleteResponse>> {
		todo!("TODO: implement TranscodingConfig::delete")
	}

	async fn tag(&self, request: Request<TranscodingConfigTagRequest>) -> Result<Response<TranscodingConfigTagResponse>> {
		let global = get_global(&self.global)?;

		let access_token =
			validate_auth_request(&global, &request, (Resource::TranscodingConfig, Permission::Modify)).await?;

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
			"transcoding_configs",
			&tags.tags,
			id,
			Some(access_token.organization_id.into()),
		)
		.await?
		.ok_or_else(|| Status::not_found("room not found"))?;

		Ok(Response::new(TranscodingConfigTagResponse {
			tags: Some(Tags { tags: updated_tags }),
		}))
	}

	async fn untag(
		&self,
		request: Request<TranscodingConfigUntagRequest>,
	) -> Result<Response<TranscodingConfigUntagResponse>> {
		let global = get_global(&self.global)?;

		let access_token =
			validate_auth_request(&global, &request, (Resource::TranscodingConfig, Permission::Modify)).await?;

		if request.get_ref().tags.is_empty() {
			return Err(Status::invalid_argument("tags must not be empty"));
		}

		let id = request.get_ref().id.to_ulid();

		let updated_tags = remove_tag_query(
			&global,
			"transcoding_configs",
			&request.get_ref().tags,
			id,
			Some(access_token.organization_id.into()),
		)
		.await?
		.ok_or_else(|| Status::not_found("recording config not found"))?;

		Ok(Response::new(TranscodingConfigUntagResponse {
			tags: Some(Tags { tags: updated_tags }),
		}))
	}
}
