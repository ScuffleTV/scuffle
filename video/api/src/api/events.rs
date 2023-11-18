use std::pin::Pin;
use std::sync::{Arc, Weak};

use futures_util::Stream;
use pb::scuffle::video::v1::events_server::{Events, EventsServer as EventsService};
use pb::scuffle::video::v1::{EventsSubscribeRequest, EventsSubscribeResponse};
use tonic::{async_trait, Request, Response, Status, Streaming};

use crate::global::ApiGlobal;

type Result<T> = std::result::Result<T, Status>;

pub struct EventsServer<G: ApiGlobal> {
	#[allow(dead_code)]
	global: Weak<G>,
}

impl<G: ApiGlobal> EventsServer<G> {
	pub fn new(global: &Arc<G>) -> EventsService<Self> {
		EventsService::new(Self {
			global: Arc::downgrade(global),
		})
	}
}

#[async_trait]
impl<G: ApiGlobal> Events for EventsServer<G> {
	type SubscribeStream = Pin<Box<dyn Stream<Item = Result<EventsSubscribeResponse>> + Send>>;

	async fn subscribe(
		&self,
		_request: Request<Streaming<EventsSubscribeRequest>>,
	) -> Result<Response<Self::SubscribeStream>> {
		todo!("TODO: implement Events::subscribe")
	}
}
