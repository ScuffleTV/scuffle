use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct Model {
    /// The unique identifier for the session.
    pub id: Uuid,
    /// Foreign key to the user table.
    pub user_id: Uuid,
    /// The time the session was invalidated.
    pub expires_at: DateTime<Utc>,
    /// The time the session was last used.
    pub last_used_at: DateTime<Utc>,
}

impl Model {
    pub fn is_valid(&self) -> bool {
        self.expires_at > Utc::now()
    }
}
