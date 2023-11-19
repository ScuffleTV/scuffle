use std::sync::{Arc, Weak};

use pb::scuffle::video::v1::recording_config_server::{
	RecordingConfig as RecordingConfigServiceTrait, RecordingConfigServer as RecordingConfigService,
};
use pb::scuffle::video::v1::{
	RecordingConfigCreateRequest, RecordingConfigCreateResponse, RecordingConfigDeleteRequest,
	RecordingConfigDeleteResponse, RecordingConfigGetRequest, RecordingConfigGetResponse, RecordingConfigModifyRequest,
	RecordingConfigModifyResponse, RecordingConfigTagRequest, RecordingConfigTagResponse, RecordingConfigUntagRequest,
	RecordingConfigUntagResponse,
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

pub struct RecordingConfigServer<G: ApiGlobal> {
	global: Weak<G>,
}

impl<G: ApiGlobal> RecordingConfigServer<G> {
	pub fn new(global: &Arc<G>) -> RecordingConfigService<Self> {
		RecordingConfigService::new(Self {
			global: Arc::downgrade(global),
		})
	}
}

#[async_trait]
impl<G: ApiGlobal> RecordingConfigServiceTrait for RecordingConfigServer<G> {
	async fn get(&self, request: Request<RecordingConfigGetRequest>) -> tonic::Result<Response<RecordingConfigGetResponse>> {
		scope_ratelimit!(self, request, global, access_token, || async {
			request.process(&global, &access_token).await
		});
	}

	async fn create(
		&self,
		request: Request<RecordingConfigCreateRequest>,
	) -> tonic::Result<Response<RecordingConfigCreateResponse>> {
		scope_ratelimit!(self, request, global, access_token, || async {
			request.process(&global, &access_token).await
		});
	}

	async fn modify(
		&self,
		request: Request<RecordingConfigModifyRequest>,
	) -> tonic::Result<Response<RecordingConfigModifyResponse>> {
		scope_ratelimit!(self, request, global, access_token, || async {
			request.process(&global, &access_token).await
		});
	}

	async fn delete(
		&self,
		request: Request<RecordingConfigDeleteRequest>,
	) -> tonic::Result<Response<RecordingConfigDeleteResponse>> {
		scope_ratelimit!(self, request, global, access_token, || async {
			request.process(&global, &access_token).await
		});
	}

	async fn tag(&self, request: Request<RecordingConfigTagRequest>) -> tonic::Result<Response<RecordingConfigTagResponse>> {
		scope_ratelimit!(self, request, global, access_token, || async {
			request.process(&global, &access_token).await
		});
	}

	async fn untag(
		&self,
		request: Request<RecordingConfigUntagRequest>,
	) -> tonic::Result<Response<RecordingConfigUntagResponse>> {
		scope_ratelimit!(self, request, global, access_token, || async {
			request.process(&global, &access_token).await
		});
	}
}
