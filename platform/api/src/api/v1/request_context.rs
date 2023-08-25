use std::{collections::HashMap, sync::Arc};

use crate::database::{role, session, user};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{api::v1::gql::error::Result, global::GlobalState};

#[derive(Clone)]
pub struct AuthData {
    pub session: session::Model,
    pub user_roles: Vec<role::Model>,
    pub user_permissions: role::Permission,
}

impl AuthData {
    pub async fn from_session_and_user(
        global: &Arc<GlobalState>,
        session: session::Model,
        user: user::Model,
    ) -> Result<Self, &'static str> {
        let global_state = global
            .global_state_loader
            .load_one(())
            .await
            .ok()
            .flatten()
            .ok_or("failed to fetch global state")?;

        let mut user_roles: Vec<role::Model> = global
            .role_by_id_loader
            .load_many(user.roles)
            .await
            .map_err(|_| "failed to fetch roles")?
            .into_values()
            .collect();

        // Computing user roles and permissions
        let global_roles_order = global_state
            .role_order
            .into_iter()
            .enumerate()
            .map(|(i, u)| (u, i))
            .collect::<HashMap<_, _>>();

        // Sort the user roles by index in global role order
        // Roles that are not included in the global role order list are considered smallest
        user_roles.sort_by_key(|r| global_roles_order.get(&r.id));

        let mut user_permissions = global_state.default_permissions;
        for role in &user_roles {
            user_permissions = user_permissions.merge_with_role(role);
        }

        Ok(Self {
            session,
            user_roles,
            user_permissions,
        })
    }

    pub async fn from_session(
        global: &Arc<GlobalState>,
        session: session::Model,
    ) -> Result<Self, &'static str> {
        let user = global
            .user_by_id_loader
            .load_one(session.user_id)
            .await
            .map_err(|_| "failed to fetch user")?
            .ok_or("user not found")?;

        Self::from_session_and_user(global, session, user).await
    }

    pub async fn from_session_id(
        global: &Arc<GlobalState>,
        session_id: Uuid,
    ) -> Result<Self, &'static str> {
        let session = global
            .session_by_id_loader
            .load_one(session_id)
            .await
            .map_err(|_| "failed to fetch session")?
            .and_then(|s| s.is_valid().then_some(s))
            .ok_or("session is no longer valid")?;

        Self::from_session(global, session).await
    }
}

#[derive(Default)]
pub struct ContextData {
    pub auth: Option<AuthData>,
}

#[derive(Default, Clone)]
pub struct RequestContext(Arc<RwLock<ContextData>>);

impl RequestContext {
    pub async fn set_auth(&self, data: AuthData) {
        let mut guard = self.0.write().await;
        guard.auth = Some(data);
    }

    pub async fn reset_auth(&self) {
        let mut guard = self.0.write().await;
        guard.auth = None;
    }

    pub async fn auth(&self) -> Option<AuthData> {
        let guard = self.0.read().await;
        guard.auth.clone()
    }
}
