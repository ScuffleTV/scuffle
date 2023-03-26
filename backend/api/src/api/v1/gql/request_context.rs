use std::sync::Arc;

use crate::database::session;
use arc_swap::ArcSwap;

use crate::{
    api::v1::gql::error::Result, dataloader::user_permissions::UserPermission, global::GlobalState,
};

use super::error::{GqlError, ResultExt};

#[derive(Default)]
pub struct RequestContext {
    is_websocket: bool,
    session: ArcSwap<Option<(session::Model, UserPermission)>>,
}

impl RequestContext {
    pub fn new(is_websocket: bool) -> Self {
        Self {
            is_websocket,
            ..Default::default()
        }
    }

    pub fn set_session(&self, session: Option<(session::Model, UserPermission)>) {
        self.session.store(Arc::new(session));
    }

    pub async fn get_session(
        &self,
        global: &Arc<GlobalState>,
    ) -> Result<Option<(session::Model, UserPermission)>> {
        let guard = self.session.load();
        let Some(session) = guard.as_ref() else {
            return Ok(None)
        };

        if !self.is_websocket {
            if !session.0.is_valid() {
                return Err(GqlError::InvalidSession.with_message("Session is no longer valid"));
            }

            return Ok(Some(session.clone()));
        }

        let session = global
            .session_by_id_loader
            .load_one(session.0.id)
            .await
            .map_err_gql("failed to fetch session")?
            .and_then(|s| if s.is_valid() { Some(s) } else { None })
            .ok_or_else(|| {
                self.session.store(Arc::new(None));
                GqlError::InvalidSession.with_message("Session is no longer valid")
            })?;

        let user_permissions = global
            .user_permisions_by_id_loader
            .load_one(session.user_id)
            .await
            .map_err_gql("failed to fetch user permissions")?
            .ok_or_else(|| {
                GqlError::InternalServerError.with_message("failed to fetch user permissions")
            })?;

        self.session
            .store(Arc::new(Some((session.clone(), user_permissions.clone()))));

        Ok(Some((session, user_permissions)))
    }
}
