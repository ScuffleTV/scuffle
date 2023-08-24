use crate::global::GlobalState;
use std::sync::{Arc, Weak};

use tonic::{async_trait, Request, Response, Status};

use pb::scuffle::video::v1::{
    playback_session_server::{PlaybackSession, PlaybackSessionServer as PlaybackSessionService},
    PlaybackSessionCountRequest, PlaybackSessionCountResponse, PlaybackSessionGetRequest,
    PlaybackSessionGetResponse, PlaybackSessionRevokeRequest, PlaybackSessionRevokeResponse,
};

type Result<T> = std::result::Result<T, Status>;

/// PlaybackSession is a session representing a user watching a video.
/// This is useful for analytics and for revoking playback sessions.
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
    /// Get returns playback sessions for a target or for users, or direct ids.
    async fn get(
        &self,
        _request: Request<PlaybackSessionGetRequest>,
    ) -> Result<Response<PlaybackSessionGetResponse>> {
        todo!("TODO: implement PlaybackSession::get")
    }

    /// Revoke revokes playback sessions for a target or for users, or direct ids.
    async fn revoke(
        &self,
        _request: Request<PlaybackSessionRevokeRequest>,
    ) -> Result<Response<PlaybackSessionRevokeResponse>> {
        todo!("TODO: implement PlaybackSession::revoke")
    }

    /// Count returns the number of playback sessions for a target.
    async fn count(
        &self,
        _request: Request<PlaybackSessionCountRequest>,
    ) -> Result<Response<PlaybackSessionCountResponse>> {
        todo!("TODO: implement PlaybackSession::count")
    }
}
