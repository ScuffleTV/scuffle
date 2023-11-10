use common::database::Ulid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SessionTokenRerevoke {
    pub organization_id: Ulid,
    pub room_id: Option<Ulid>,
    pub recording_id: Option<Ulid>,
    pub user_id: Option<String>,
    pub sso_id: Option<String>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}
