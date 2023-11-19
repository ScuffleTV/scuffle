use std::sync::{Arc, Weak};

use pb::scuffle::video::v1::transcoding_config_server::{
	TranscodingConfig as TranscodingConfigTrait, TranscodingConfigServer as TranscodingConfigService,
};
use pb::scuffle::video::v1::{
	TranscodingConfigCreateRequest, TranscodingConfigCreateResponse, TranscodingConfigDeleteRequest,
	TranscodingConfigDeleteResponse, TranscodingConfigGetRequest, TranscodingConfigGetResponse,
	TranscodingConfigModifyRequest, TranscodingConfigModifyResponse, TranscodingConfigTagRequest,
	TranscodingConfigTagResponse, TranscodingConfigUntagRequest, TranscodingConfigUntagResponse,
};
use tonic::{async_trait, Request, Response};

use super::utils::ratelimit::scope_ratelimit;
use super::utils::ApiRequest;
use crate::global::ApiGlobal;

mod create;
mod delete;
mod get;
mod modify;
mod tag;
mod untag;

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
	async fn get(
		&self,
		request: Request<TranscodingConfigGetRequest>,
	) -> tonic::Result<Response<TranscodingConfigGetResponse>> {
		scope_ratelimit!(self, request, global, access_token, || async {
			request.process(&global, &access_token).await
		});
	}

	async fn create(
		&self,
		request: Request<TranscodingConfigCreateRequest>,
	) -> tonic::Result<Response<TranscodingConfigCreateResponse>> {
		scope_ratelimit!(self, request, global, access_token, || async {
			request.process(&global, &access_token).await
		});
	}

	async fn modify(
		&self,
		request: Request<TranscodingConfigModifyRequest>,
	) -> tonic::Result<Response<TranscodingConfigModifyResponse>> {
		scope_ratelimit!(self, request, global, access_token, || async {
			request.process(&global, &access_token).await
		});
	}

	async fn delete(
		&self,
		request: Request<TranscodingConfigDeleteRequest>,
	) -> tonic::Result<Response<TranscodingConfigDeleteResponse>> {
		scope_ratelimit!(self, request, global, access_token, || async {
			request.process(&global, &access_token).await
		});
	}

	async fn tag(
		&self,
		request: Request<TranscodingConfigTagRequest>,
	) -> tonic::Result<Response<TranscodingConfigTagResponse>> {
		scope_ratelimit!(self, request, global, access_token, || async {
			request.process(&global, &access_token).await
		});
	}

	async fn untag(
		&self,
		request: Request<TranscodingConfigUntagRequest>,
	) -> tonic::Result<Response<TranscodingConfigUntagResponse>> {
		scope_ratelimit!(self, request, global, access_token, || async {
			request.process(&global, &access_token).await
		});
	}
}
