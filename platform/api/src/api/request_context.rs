use std::ops::Deref;
use std::sync::Arc;

use tokio::sync::RwLock;

use super::auth::{AuthData, AuthError};
use crate::global::ApiGlobal;

#[derive(Default, Clone)]
pub struct ContextData {
	pub auth: Option<AuthData>,
	pub websocket: bool,
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

	pub async fn websocket(&self) {
		let mut guard = self.0.write().await;
		guard.websocket = true;
	}

	pub async fn auth<G: ApiGlobal>(&self, global: &Arc<G>) -> Result<Option<AuthData>, AuthError> {
		let inner = self.0.read().await.deref().clone();
		match inner.auth {
			Some(auth) => {
				if !auth.session.is_valid() {
					Err(AuthError::SessionExpired)
				} else if inner.websocket {
					let auth = AuthData::from_session_id(global, auth.session.id.0).await?;
					if auth.session.is_valid() {
						self.set_auth(auth.clone()).await;
						Ok(Some(auth))
					} else {
						self.reset_auth().await;
						Err(AuthError::SessionExpired)
					}
				} else {
					Ok(Some(auth))
				}
			}
			None => Ok(None),
		}
	}
}
