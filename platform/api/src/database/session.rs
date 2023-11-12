use chrono::{DateTime, Utc};
use common::database::Ulid;

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct Session {
    /// The unique identifier for the session.
    pub id: Ulid,
    /// Foreign key to the user table.
    pub user_id: Ulid,
    /// The time the session was invalidated.
    pub expires_at: DateTime<Utc>,
    /// The time the session was last used.
    pub last_used_at: DateTime<Utc>,
}

impl Session {
    pub fn is_valid(&self) -> bool {
        self.expires_at > Utc::now()
    }
}
