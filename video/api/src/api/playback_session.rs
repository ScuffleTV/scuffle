use crate::global::GlobalState;
use std::sync::{Arc, Weak};

use tonic::{async_trait, Request, Response, Status};

use pb::scuffle::video::v1::{
    playback_session_server::{PlaybackSession, PlaybackSessionServer as PlaybackSessionService},
    PlaybackSessionCountRequest, PlaybackSessionCountResponse, PlaybackSessionGetRequest,
    PlaybackSessionGetResponse, PlaybackSessionRevokeRequest, PlaybackSessionRevokeResponse,
};

type Result<T> = std::result::Result<T, Status>;

pub struct PlaybackSessionServer {
    global: Weak<GlobalState>,
}

impl PlaybackSessionServer {
    pub fn new(global: &Arc<GlobalState>) -> PlaybackSessionService<Self> {
        PlaybackSessionService::new(Self {
            global: Arc::downgrade(global),
        })
    }
}

#[async_trait]
impl PlaybackSession for PlaybackSessionServer {
    async fn get(
        &self,
        _request: Request<PlaybackSessionGetRequest>,
    ) -> Result<Response<PlaybackSessionGetResponse>> {
        todo!("TODO: implement PlaybackSession::get")
    }

    async fn revoke(
        &self,
        _request: Request<PlaybackSessionRevokeRequest>,
    ) -> Result<Response<PlaybackSessionRevokeResponse>> {
        todo!("TODO: implement PlaybackSession::revoke")
    }

    async fn count(
        &self,
        _request: Request<PlaybackSessionCountRequest>,
    ) -> Result<Response<PlaybackSessionCountResponse>> {
        todo!("TODO: implement PlaybackSession::count")
    }
}
