use std::{collections::HashMap, sync::Arc};

use bytes::Bytes;
use tokio::sync::{mpsc, RwLock};
use transmuxer::MediaSegment;
use uuid::Uuid;

pub struct StreamConnection {
    connection_id: Uuid,
    channel: mpsc::Sender<GrpcRequest>,
}

pub enum GrpcRequest {
    Started {
        id: Uuid,
    },
    WatchStream {
        id: Uuid,
        channel: mpsc::Sender<WatchStreamEvent>,
    },
    ShuttingDown {
        id: Uuid,
    },
    Error {
        id: Uuid,
        message: String,
        fatal: bool,
    },
}

#[derive(Debug)]
pub enum WatchStreamEvent {
    InitSegment(Bytes),
    MediaSegment(MediaSegment),
    ShuttingDown(bool),
}

pub struct StreamManager {
    streams: RwLock<HashMap<Uuid, Arc<StreamConnection>>>,
}

impl StreamManager {
    pub fn new() -> Self {
        Self {
            streams: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register_stream(
        &self,
        stream_id: Uuid,
        connection_id: Uuid,
        channel: mpsc::Sender<GrpcRequest>,
    ) {
        let mut streams = self.streams.write().await;

        streams.insert(
            stream_id,
            Arc::new(StreamConnection {
                connection_id,
                channel,
            }),
        );
    }

    pub async fn deregister_stream(&self, stream_id: Uuid, connection_id: Uuid) {
        let mut streams = self.streams.write().await;

        let connection = streams.get(&stream_id);

        if let Some(connection) = connection {
            if connection.connection_id == connection_id {
                streams.remove(&stream_id);
            }
        }
    }

    pub async fn submit_request(&self, stream_id: Uuid, request: GrpcRequest) -> bool {
        let connections = self.streams.read().await;

        let Some(connection) = connections.get(&stream_id).cloned() else {
            return false;
        };

        // We dont want to hold the lock while we wait for the channel to be ready
        drop(connections);

        // We dont care if this fails since if it does fail,
        // the channel will be dropped and therefore it will report
        // to the caller that the stream is no longer available.
        connection.channel.send(request).await.is_ok()
    }
}
