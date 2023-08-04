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

/// The Events service provides a stream of events that occur in the system.
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
    /// Server streaming response type for the Subscribe method.
    type SubscribeStream = Pin<Box<dyn Stream<Item = Result<EventsSubscribeResponse>> + Send>>;

    /// Subscribe to events. The client should send an `OnOpen` event to
    /// indicate that it is ready to receive events. The server will respond
    /// with Events. The client should send an `AckEvent` event to indicate
    /// that it has processed the event. If the client does not send an
    /// `AckEvent` event, the server will resend the event after a timeout.
    async fn subscribe(
        &self,
        _request: Request<Streaming<EventsSubscribeRequest>>,
    ) -> Result<Response<Self::SubscribeStream>> {
        todo!("TODO: implement Events::subscribe")
    }
}
