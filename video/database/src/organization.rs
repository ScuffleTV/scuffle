use uuid::Uuid;

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct Organization {
    // The primary key for the organization
    pub id: Uuid,

    // The date and time the organization was last updated
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
