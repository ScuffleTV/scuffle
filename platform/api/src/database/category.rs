use chrono::{DateTime, Utc};

use common::database::Ulid;

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct Category {
    pub id: Ulid,
    pub name: String,
    pub revision: i32,
    pub updated_at: DateTime<Utc>,
}
