use std::sync::{Arc, Weak};

use pb::scuffle::video::v1::playback_session_server::{
	PlaybackSession as PlaybackSessionServiceTrait, PlaybackSessionServer as PlaybackSessionService,
};
use pb::scuffle::video::v1::{
	PlaybackSessionCountRequest, PlaybackSessionCountResponse, PlaybackSessionGetRequest, PlaybackSessionGetResponse,
	PlaybackSessionRevokeRequest, PlaybackSessionRevokeResponse,
};
use tonic::{async_trait, Request, Response};

use super::utils::ratelimit::scope_ratelimit;
use super::utils::ApiRequest;
use crate::global::ApiGlobal;

mod count;
mod get;
mod revoke;

pub struct PlaybackSessionServer<G: ApiGlobal> {
	global: Weak<G>,
}

impl<G: ApiGlobal> PlaybackSessionServer<G> {
	pub fn new(global: &Arc<G>) -> PlaybackSessionService<Self> {
		PlaybackSessionService::new(Self {
			global: Arc::downgrade(global),
		})
	}
}

#[async_trait]
impl<G: ApiGlobal> PlaybackSessionServiceTrait for PlaybackSessionServer<G> {
	async fn get(&self, request: Request<PlaybackSessionGetRequest>) -> tonic::Result<Response<PlaybackSessionGetResponse>> {
		scope_ratelimit!(self, request, global, access_token, || async {
			request.process(&global, &access_token).await
		});
	}

	async fn revoke(
		&self,
		request: Request<PlaybackSessionRevokeRequest>,
	) -> tonic::Result<Response<PlaybackSessionRevokeResponse>> {
		scope_ratelimit!(self, request, global, access_token, || async {
			request.process(&global, &access_token).await
		});
	}

	async fn count(
		&self,
		request: Request<PlaybackSessionCountRequest>,
	) -> tonic::Result<Response<PlaybackSessionCountResponse>> {
		scope_ratelimit!(self, request, global, access_token, || async {
			request.process(&global, &access_token).await
		});
	}
}
