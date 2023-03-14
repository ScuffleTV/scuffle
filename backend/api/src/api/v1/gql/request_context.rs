use std::sync::Arc;

use arc_swap::ArcSwap;
use common::types::session;

use crate::{api::v1::gql::error::Result, global::GlobalState};

use super::error::{GqlError, ResultExt};

#[derive(Default)]
pub struct RequestContext {
    is_websocket: bool,
    session: ArcSwap<Option<session::Model>>,
}

impl RequestContext {
    pub fn new(is_websocket: bool) -> Self {
        Self {
            is_websocket,
            ..Default::default()
        }
    }

    pub fn set_session(&self, session: Option<session::Model>) {
        self.session.store(Arc::new(session));
    }

    pub fn is_websocket(&self) -> bool {
        self.is_websocket
    }

    pub async fn get_session(&self, global: &Arc<GlobalState>) -> Result<Option<session::Model>> {
        let guard = self.session.load();
        let Some(session) = guard.as_ref() else {
            return Ok(None)
        };

        if !self.is_websocket {
            if !session.is_valid() {
                return Err(GqlError::InvalidSession.with_message("Session is no longer valid"));
            }

            return Ok(Some(session.clone()));
        }

        let session = global
            .session_by_id_loader
            .load_one(session.id)
            .await
            .map_err_gql("failed to fetch session")?
            .and_then(|s| if s.is_valid() { Some(s) } else { None })
            .ok_or_else(|| {
                self.session.store(Arc::new(None));
                GqlError::InvalidSession.with_message("Session is no longer valid")
            })?;

        self.session.store(Arc::new(Some(session.clone())));

        Ok(Some(session))
    }
}
