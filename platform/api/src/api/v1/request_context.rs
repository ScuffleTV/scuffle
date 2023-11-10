use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;
use ulid::Ulid;

use crate::{
    api::{middleware::auth::AuthError, v1::gql::error::Result},
    database::{Role, RolePermission, Session, User},
    global::GlobalState,
};

#[derive(Clone)]
pub struct AuthData {
    pub session: Session,
    pub user_roles: Vec<Role>,
    pub user_permissions: RolePermission,
}

impl AuthData {
    pub async fn from_session_and_user(
        global: &Arc<GlobalState>,
        session: Session,
        user: &User,
    ) -> Result<Self, &'static str> {
        let global_state = global
            .global_state_loader
            .load(())
            .await
            .ok()
            .flatten()
            .ok_or("failed to fetch global state")?;

        let mut user_roles: Vec<Role> = global
            .role_by_id_loader
            .load_many(user.roles.iter().map(|i| i.0))
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
        session: Session,
    ) -> Result<Self, &'static str> {
        let user = global
            .user_by_id_loader
            .load(session.user_id.0)
            .await
            .map_err(|_| "failed to fetch user")?
            .ok_or("user not found")?;

        Self::from_session_and_user(global, session, &user).await
    }

    pub async fn from_session_id(
        global: &Arc<GlobalState>,
        session_id: Ulid,
    ) -> Result<Self, &'static str> {
        // TODO: Return proper error
        let session = global
            .session_by_id_loader
            .load(session_id)
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

    pub async fn auth(&self) -> Result<Option<AuthData>, AuthError> {
        match self.0.read().await.auth.clone() {
            Some(auth) => {
                // TODO: Refetch session from db?
                if !auth.session.is_valid() {
                    Err(AuthError::InvalidToken)
                } else {
                    Ok(Some(auth))
                }
            }
            None => Ok(None),
        }
    }
}
