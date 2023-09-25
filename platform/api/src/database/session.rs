use super::Ulid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct Session {
    /// The unique identifier for the session.
    pub id: Ulid,
    /// Foreign key to the user table.
    pub user_id: Ulid,
    /// Whether the user has solved the two-factor authentication challenge.
    pub two_fa_solved: bool,
    /// The time the session was invalidated.
    pub expires_at: DateTime<Utc>,
    /// The time the session was last used.
    pub last_used_at: DateTime<Utc>,
}

impl Session {
    pub fn is_two_fa_solved(&self) -> bool {
        self.two_fa_solved
    }

    pub fn is_valid(&self) -> bool {
        self.expires_at > Utc::now()
    }
}
