use crate::global::GlobalState;
use std::sync::{Arc, Weak};

use tonic::{async_trait, Request, Response, Status};

use pb::scuffle::video::v1::{
    recording_server::{Recording, RecordingServer as RecordingService},
    RecordingDeleteRequest, RecordingDeleteResponse, RecordingGetRequest, RecordingGetResponse,
    RecordingModifyRequest, RecordingModifyResponse,
};

type Result<T> = std::result::Result<T, Status>;

/// A recording is a video that was recorded in a room.
/// It can be public or private and it is managed by lifecycle policies.
/// You can start recording rooms by attaching a RecordingConfig to a room.
pub struct RecordingServer {
    global: Weak<GlobalState>,
}

impl RecordingServer {
    pub fn new(global: &Arc<GlobalState>) -> RecordingService<Self> {
        RecordingService::new(Self {
            global: Arc::downgrade(global),
        })
    }
}

#[async_trait]
impl Recording for RecordingServer {
    /// Modify recordings.
    async fn modify(
        &self,
        _request: Request<RecordingModifyRequest>,
    ) -> Result<Response<RecordingModifyResponse>> {
        todo!("TODO: implement Recording::modify")
    }

    /// Get recordings.
    async fn get(
        &self,
        _request: Request<RecordingGetRequest>,
    ) -> Result<Response<RecordingGetResponse>> {
        todo!("TODO: implement Recording::get")
    }

    /// Delete recordings.
    async fn delete(
        &self,
        _request: Request<RecordingDeleteRequest>,
    ) -> Result<Response<RecordingDeleteResponse>> {
        todo!("TODO: implement Recording::delete")
    }
}
