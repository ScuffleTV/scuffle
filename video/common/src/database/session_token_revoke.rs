use common::database::Ulid;

use super::DatabaseTable;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SessionTokenRevoke {
	pub organization_id: Ulid,
	pub room_id: Option<Ulid>,
	pub recording_id: Option<Ulid>,
	pub user_id: Option<String>,
	pub sso_id: Option<String>,
	pub expires_at: chrono::DateTime<chrono::Utc>,
}

impl DatabaseTable for SessionTokenRevoke {
	const FRIENDLY_NAME: &'static str = "session token revoke";
	const NAME: &'static str = "session_token_revokes";
}
