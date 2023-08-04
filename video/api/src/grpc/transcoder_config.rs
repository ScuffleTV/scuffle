use crate::global::GlobalState;
use std::sync::{Arc, Weak};

use tonic::{async_trait, Request, Response, Status};

use pb::scuffle::video::v1::{
    transcoder_config_server::{
        TranscoderConfig, TranscoderConfigServer as TranscoderConfigService,
    },
    TranscoderConfigDeleteRequest, TranscoderConfigDeleteResponse, TranscoderConfigGetRequest,
    TranscoderConfigGetResponse, TranscoderConfigModifyRequest, TranscoderConfigModifyResponse,
};

type Result<T> = std::result::Result<T, Status>;

/// TranscoderConfig is a service for managing transcoder configs.
/// Transcoder configs define how rooms will be transcoded, and what renditions
/// will be available. You can attch a transcoder config to a room by calling
/// Room.Modify with the transcoder_config_name field.
pub struct TranscoderConfigServer {
    global: Weak<GlobalState>,
}

impl TranscoderConfigServer {
    pub fn new(global: &Arc<GlobalState>) -> TranscoderConfigService<Self> {
        TranscoderConfigService::new(Self {
            global: Arc::downgrade(global),
        })
    }
}

#[async_trait]
impl TranscoderConfig for TranscoderConfigServer {
    /// Modify allows you to create a new transcoder config, or update an existing
    /// one.
    async fn modify(
        &self,
        _request: Request<TranscoderConfigModifyRequest>,
    ) -> Result<Response<TranscoderConfigModifyResponse>> {
        todo!("TODO: implement TranscoderConfig::modify")
    }

    /// Get allows you to get a transcoder configs.
    async fn get(
        &self,
        _request: Request<TranscoderConfigGetRequest>,
    ) -> Result<Response<TranscoderConfigGetResponse>> {
        todo!("TODO: implement TranscoderConfig::get")
    }

    /// Delete allows you to delete multiple transcoder configs.
    async fn delete(
        &self,
        _request: Request<TranscoderConfigDeleteRequest>,
    ) -> Result<Response<TranscoderConfigDeleteResponse>> {
        todo!("TODO: implement TranscoderConfig::delete")
    }
}
