use pb::scuffle::video::internal::{IngestWatchRequest, IngestWatchResponse};
use tokio::sync::mpsc;
use tonic::Streaming;
use ulid::Ulid;

pub struct IncomingTranscoder {
    pub ulid: Ulid,
    pub message: IngestWatchRequest,
    pub streaming: Streaming<IngestWatchRequest>,
    pub transcoder: mpsc::Sender<IngestWatchResponse>,
}
