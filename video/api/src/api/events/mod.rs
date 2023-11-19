use std::pin::Pin;
use std::sync::{Arc, Weak};

use futures_util::Stream;
use pb::scuffle::video::v1::events_server::{Events, EventsServer as EventsService};
use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::Resource;
use pb::scuffle::video::v1::{EventsSubscribeRequest, EventsSubscribeResponse};
use tonic::{async_trait, Request, Response, Streaming};

use super::utils::impl_request_scopes;
use super::utils::ratelimit::scope_ratelimit;
use crate::global::ApiGlobal;
use crate::ratelimit::RateLimitResource;

pub struct EventsServer<G: ApiGlobal> {
	#[allow(dead_code)]
	global: Weak<G>,
}

impl_request_scopes!(
	Streaming<EventsSubscribeRequest>,
	(),
	(Resource::Event, Permission::Read),
	RateLimitResource::EventsSubscribe
);

impl<G: ApiGlobal> EventsServer<G> {
	pub fn new(global: &Arc<G>) -> EventsService<Self> {
		EventsService::new(Self {
			global: Arc::downgrade(global),
		})
	}
}

#[async_trait]
impl<G: ApiGlobal> Events for EventsServer<G> {
	type SubscribeStream = Pin<Box<dyn Stream<Item = tonic::Result<EventsSubscribeResponse>> + Send + Sync>>;

	async fn subscribe(
		&self,
		request: Request<Streaming<EventsSubscribeRequest>>,
	) -> tonic::Result<Response<Self::SubscribeStream>> {
		scope_ratelimit!(self, request, global, access_token, || async {
			todo!("TODO: implement Events::subscribe")
		});
	}
}
