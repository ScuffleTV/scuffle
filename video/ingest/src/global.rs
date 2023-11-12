use std::collections::HashMap;

use crate::config::IngestConfig;
use pb::scuffle::video::internal::{IngestWatchRequest, IngestWatchResponse};
use tokio::sync::{mpsc, Mutex};
use tonic::Streaming;
use ulid::Ulid;

pub struct IncomingTranscoder {
    pub ulid: Ulid,
    pub message: IngestWatchRequest,
    pub streaming: Streaming<IngestWatchRequest>,
    pub transcoder: mpsc::Sender<IngestWatchResponse>,
}

pub trait IngestState {
    fn requests(&self) -> &Mutex<HashMap<Ulid, mpsc::Sender<IncomingTranscoder>>>;
}

pub trait IngestGlobal:
    common::global::GlobalCtx
    + common::global::GlobalConfigProvider<IngestConfig>
    + common::global::GlobalNats
    + common::global::GlobalDb
    + common::global::GlobalConfig
    + IngestState
    + Send
    + Sync
    + 'static
{
}

impl<T> IngestGlobal for T where
    T: common::global::GlobalCtx
        + common::global::GlobalConfigProvider<IngestConfig>
        + common::global::GlobalNats
        + common::global::GlobalDb
        + common::global::GlobalConfig
        + IngestState
        + Send
        + Sync
        + 'static
{
}
