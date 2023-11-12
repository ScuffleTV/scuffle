use crate::global::ApiGlobal;
use std::sync::{Arc, Weak};

use tonic::{async_trait, Request, Response, Status};

use pb::scuffle::video::v1::{
    playback_session_server::{PlaybackSession, PlaybackSessionServer as PlaybackSessionService},
    PlaybackSessionCountRequest, PlaybackSessionCountResponse, PlaybackSessionGetRequest,
    PlaybackSessionGetResponse, PlaybackSessionRevokeRequest, PlaybackSessionRevokeResponse,
};

type Result<T> = std::result::Result<T, Status>;

pub struct PlaybackSessionServer<G: ApiGlobal> {
    global: Weak<G>,
}

impl<G: ApiGlobal> PlaybackSessionServer<G> {
    pub fn new(global: &Arc<G>) -> PlaybackSessionService<Self> {
        PlaybackSessionService::new(Self {
            global: Arc::downgrade(global),
        })
    }
}

#[async_trait]
impl<G: ApiGlobal> PlaybackSession for PlaybackSessionServer<G> {
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
