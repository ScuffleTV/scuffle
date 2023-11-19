use std::sync::{Arc, Weak};

use pb::scuffle::video::v1::playback_key_pair_server::{
	PlaybackKeyPair as PlaybackKeyPairServiceTrait, PlaybackKeyPairServer as PlaybackKeyPairService,
};
use pb::scuffle::video::v1::{
	PlaybackKeyPairCreateRequest, PlaybackKeyPairCreateResponse, PlaybackKeyPairDeleteRequest,
	PlaybackKeyPairDeleteResponse, PlaybackKeyPairGetRequest, PlaybackKeyPairGetResponse, PlaybackKeyPairModifyRequest,
	PlaybackKeyPairModifyResponse, PlaybackKeyPairTagRequest, PlaybackKeyPairTagResponse, PlaybackKeyPairUntagRequest,
	PlaybackKeyPairUntagResponse,
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

mod utils;

pub struct PlaybackKeyPairServer<G: ApiGlobal> {
	global: Weak<G>,
}

impl<G: ApiGlobal> PlaybackKeyPairServer<G> {
	pub fn new(global: &Arc<G>) -> PlaybackKeyPairService<Self> {
		PlaybackKeyPairService::new(Self {
			global: Arc::downgrade(global),
		})
	}
}

#[async_trait]
impl<G: ApiGlobal> PlaybackKeyPairServiceTrait for PlaybackKeyPairServer<G> {
	async fn get(&self, request: Request<PlaybackKeyPairGetRequest>) -> tonic::Result<Response<PlaybackKeyPairGetResponse>> {
		scope_ratelimit!(self, request, global, access_token, || async {
			request.process(&global, &access_token).await
		});
	}

	async fn create(
		&self,
		request: Request<PlaybackKeyPairCreateRequest>,
	) -> tonic::Result<Response<PlaybackKeyPairCreateResponse>> {
		scope_ratelimit!(self, request, global, access_token, || async {
			request.process(&global, &access_token).await
		});
	}

	async fn modify(
		&self,
		request: Request<PlaybackKeyPairModifyRequest>,
	) -> tonic::Result<Response<PlaybackKeyPairModifyResponse>> {
		scope_ratelimit!(self, request, global, access_token, || async {
			request.process(&global, &access_token).await
		});
	}

	async fn delete(
		&self,
		request: Request<PlaybackKeyPairDeleteRequest>,
	) -> tonic::Result<Response<PlaybackKeyPairDeleteResponse>> {
		scope_ratelimit!(self, request, global, access_token, || async {
			request.process(&global, &access_token).await
		});
	}

	async fn tag(&self, request: Request<PlaybackKeyPairTagRequest>) -> tonic::Result<Response<PlaybackKeyPairTagResponse>> {
		scope_ratelimit!(self, request, global, access_token, || async {
			request.process(&global, &access_token).await
		});
	}

	async fn untag(
		&self,
		request: Request<PlaybackKeyPairUntagRequest>,
	) -> tonic::Result<Response<PlaybackKeyPairUntagResponse>> {
		scope_ratelimit!(self, request, global, access_token, || async {
			request.process(&global, &access_token).await
		});
	}
}
