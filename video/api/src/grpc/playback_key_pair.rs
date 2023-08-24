use crate::global::GlobalState;
use std::sync::{Arc, Weak};

use tonic::{async_trait, Request, Response, Status};

use pb::scuffle::video::v1::{
    playback_key_pair_server::{PlaybackKeyPair, PlaybackKeyPairServer as PlaybackKeyPairService},
    PlaybackKeyPairDeleteRequest, PlaybackKeyPairDeleteResponse, PlaybackKeyPairGetRequest,
    PlaybackKeyPairGetResponse, PlaybackKeyPairModifyRequest, PlaybackKeyPairModifyResponse,
};

type Result<T> = std::result::Result<T, Status>;

/// PlaybackKeyPair is a service for managing playback key pairs.
/// Playback key pairs are used to authenticate playback requests.
/// They are used to ensure that only authorized users can view a stream.
pub struct PlaybackKeyPairServer {
    global: Weak<GlobalState>,
}

impl PlaybackKeyPairServer {
    pub fn new(global: &Arc<GlobalState>) -> PlaybackKeyPairService<Self> {
        PlaybackKeyPairService::new(Self {
            global: Arc::downgrade(global),
        })
    }
}

#[async_trait]
impl PlaybackKeyPair for PlaybackKeyPairServer {
    /// Modifys a new playback key pair, or updates an existing one.
    async fn modify(
        &self,
        _request: Request<PlaybackKeyPairModifyRequest>,
    ) -> Result<Response<PlaybackKeyPairModifyResponse>> {
        todo!("TODO: implement PlaybackKeyPair::modify")
    }

    /// Gets playback key pairs.
    async fn get(
        &self,
        _request: Request<PlaybackKeyPairGetRequest>,
    ) -> Result<Response<PlaybackKeyPairGetResponse>> {
        todo!("TODO: implement PlaybackKeyPair::get")
    }

    /// Deletes playback key pairs.
    async fn delete(
        &self,
        _request: Request<PlaybackKeyPairDeleteRequest>,
    ) -> Result<Response<PlaybackKeyPairDeleteResponse>> {
        todo!("TODO: implement PlaybackKeyPair::delete")
    }
}
