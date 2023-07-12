use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct Model {
    /// The unique identifier for the session.
    pub id: Uuid,
    /// Foreign key to the user table.
    pub user_id: Uuid,
    /// The time the session was invalidated.
    pub invalidated_at: Option<DateTime<Utc>>,
    /// The time the session was created.
    pub created_at: DateTime<Utc>,
    /// The time the session expires.
    pub expires_at: DateTime<Utc>,
    /// The time the session was last used.
    pub last_used_at: DateTime<Utc>,
}

impl Model {
    pub fn is_valid(&self) -> bool {
        if self.invalidated_at.is_some() {
            return false;
        }

        if self.expires_at < Utc::now() {
            return false;
        }

        true
    }
}
