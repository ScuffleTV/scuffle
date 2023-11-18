use std::sync::Arc;

use tokio::sync::RwLock;

use super::auth::{AuthData, AuthError};

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
					Err(AuthError::SessionExpired)
				} else {
					Ok(Some(auth))
				}
			}
			None => Ok(None),
		}
	}
}
