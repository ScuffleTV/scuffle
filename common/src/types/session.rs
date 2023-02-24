use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Default)]
pub struct Model {
    pub id: i64,                               // bigint, primary key
    pub user_id: i64,                          // bigint, foreign key -> users.id
    pub invalidated_at: Option<DateTime<Utc>>, // timestampz
    pub created_at: DateTime<Utc>,             // timestamptz
    pub expires_at: DateTime<Utc>,             // timestamptz?
    pub last_used_at: DateTime<Utc>,           // timestamptz
}

impl Model {
    pub fn validate(&self) -> bool {
        if self.invalidated_at.is_some() {
            return false;
        }

        if self.expires_at < Utc::now() {
            return false;
        }

        true
    }
}
