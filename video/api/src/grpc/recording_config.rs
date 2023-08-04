use crate::global::GlobalState;
use std::sync::{Arc, Weak};

use tonic::{async_trait, Request, Response, Status};

use pb::scuffle::video::v1::{
    recording_config_server::{RecordingConfig, RecordingConfigServer as RecordingConfigService},
    RecordingConfigDeleteRequest, RecordingConfigDeleteResponse, RecordingConfigGetRequest,
    RecordingConfigGetResponse, RecordingConfigModifyRequest, RecordingConfigModifyResponse,
};

type Result<T> = std::result::Result<T, Status>;

/// RecordingConfig is the service for managing recording configs.
/// Recording configs are used to determine what renditions to record for a
/// stream. They also allow you to set lifecycle policies for the recordings.
/// Recording configs are applied to rooms via the room's recording_config_name.
/// If a room does not have a recording_config_name, it will not be recorded.
pub struct RecordingConfigServer {
    global: Weak<GlobalState>,
}

impl RecordingConfigServer {
    pub fn new(global: &Arc<GlobalState>) -> RecordingConfigService<Self> {
        RecordingConfigService::new(Self {
            global: Arc::downgrade(global),
        })
    }
}

#[async_trait]
impl RecordingConfig for RecordingConfigServer {
    /// Modify or update a recording config.
    async fn modify(
        &self,
        _request: Request<RecordingConfigModifyRequest>,
    ) -> Result<Response<RecordingConfigModifyResponse>> {
        todo!("TODO: implement RecordingConfig::modify")
    }

    /// Get recording configs.
    async fn get(
        &self,
        _request: Request<RecordingConfigGetRequest>,
    ) -> Result<Response<RecordingConfigGetResponse>> {
        todo!("TODO: implement RecordingConfig::get")
    }

    /// Delete recording configs.
    async fn delete(
        &self,
        _request: Request<RecordingConfigDeleteRequest>,
    ) -> Result<Response<RecordingConfigDeleteResponse>> {
        todo!("TODO: implement RecordingConfig::delete")
    }
}
