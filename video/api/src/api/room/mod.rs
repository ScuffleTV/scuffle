use std::sync::{Arc, Weak};

use pb::scuffle::video::v1::room_server::{Room as RoomServiceTrait, RoomServer as RoomService};
use pb::scuffle::video::v1::{
	RoomCreateRequest, RoomCreateResponse, RoomDeleteRequest, RoomDeleteResponse, RoomDisconnectRequest,
	RoomDisconnectResponse, RoomGetRequest, RoomGetResponse, RoomModifyRequest, RoomModifyResponse, RoomResetKeyRequest,
	RoomResetKeyResponse, RoomTagRequest, RoomTagResponse, RoomUntagRequest, RoomUntagResponse,
};
use tonic::{async_trait, Request, Response};

mod create;
mod delete;
mod disconnect;
mod get;
mod modify;
mod reset_key;
mod tag;
mod untag;

mod utils;

use super::utils::ratelimit::scope_ratelimit;
use super::utils::ApiRequest;
use crate::global::ApiGlobal;

pub struct RoomServer<G: ApiGlobal> {
	global: Weak<G>,
}

impl<G: ApiGlobal> RoomServer<G> {
	pub fn new(global: &Arc<G>) -> RoomService<Self> {
		RoomService::new(Self {
			global: Arc::downgrade(global),
		})
	}
}

#[async_trait]
impl<G: ApiGlobal> RoomServiceTrait for RoomServer<G> {
	async fn get(&self, request: Request<RoomGetRequest>) -> tonic::Result<Response<RoomGetResponse>> {
		scope_ratelimit!(self, request, global, access_token, || async {
			request.process(&global, &access_token).await
		});
	}

	async fn create(&self, request: Request<RoomCreateRequest>) -> tonic::Result<Response<RoomCreateResponse>> {
		scope_ratelimit!(self, request, global, access_token, || async {
			request.process(&global, &access_token).await
		});
	}

	async fn modify(&self, request: Request<RoomModifyRequest>) -> tonic::Result<Response<RoomModifyResponse>> {
		scope_ratelimit!(self, request, global, access_token, || async {
			request.process(&global, &access_token).await
		});
	}

	async fn delete(&self, request: Request<RoomDeleteRequest>) -> tonic::Result<Response<RoomDeleteResponse>> {
		scope_ratelimit!(self, request, global, access_token, || async {
			request.process(&global, &access_token).await
		});
	}

	async fn disconnect(&self, request: Request<RoomDisconnectRequest>) -> tonic::Result<Response<RoomDisconnectResponse>> {
		scope_ratelimit!(self, request, global, access_token, || async {
			request.process(&global, &access_token).await
		});
	}

	async fn reset_key(&self, request: Request<RoomResetKeyRequest>) -> tonic::Result<Response<RoomResetKeyResponse>> {
		scope_ratelimit!(self, request, global, access_token, || async {
			request.process(&global, &access_token).await
		});
	}

	async fn tag(&self, request: Request<RoomTagRequest>) -> tonic::Result<Response<RoomTagResponse>> {
		scope_ratelimit!(self, request, global, access_token, || async {
			request.process(&global, &access_token).await
		});
	}

	async fn untag(&self, request: Request<RoomUntagRequest>) -> tonic::Result<Response<RoomUntagResponse>> {
		scope_ratelimit!(self, request, global, access_token, || async {
			request.process(&global, &access_token).await
		});
	}
}
