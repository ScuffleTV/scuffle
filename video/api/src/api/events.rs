use crate::global::GlobalState;
use std::{
    pin::Pin,
    sync::{Arc, Weak},
};

use futures_util::Stream;
use tonic::{async_trait, Request, Response, Status, Streaming};

use pb::scuffle::video::v1::{
    events_server::{Events, EventsServer as EventsService},
    EventsSubscribeRequest, EventsSubscribeResponse,
};

type Result<T> = std::result::Result<T, Status>;

pub struct EventsServer {
    global: Weak<GlobalState>,
}

impl EventsServer {
    pub fn new(global: &Arc<GlobalState>) -> EventsService<Self> {
        EventsService::new(Self {
            global: Arc::downgrade(global),
        })
    }
}

#[async_trait]
impl Events for EventsServer {
    type SubscribeStream = Pin<Box<dyn Stream<Item = Result<EventsSubscribeResponse>> + Send>>;

    async fn subscribe(
        &self,
        _request: Request<Streaming<EventsSubscribeRequest>>,
    ) -> Result<Response<Self::SubscribeStream>> {
        todo!("TODO: implement Events::subscribe")
    }
}
