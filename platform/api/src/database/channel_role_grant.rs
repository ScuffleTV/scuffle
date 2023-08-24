use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Default, sqlx::FromRow)]
/// A grant of a channel role to a user.
/// This allows for channel owners to grant roles to other users in their channel.
/// See the `channel_role` table for more information.
pub struct Model {
    /// The unique identifier for the grant.
    pub id: Uuid,
    /// Foreign key to the user table.
    pub user_id: Uuid,
    /// Foreign key to the channel_role table.
    pub channel_role_id: Uuid,
    /// The time the grant was created.
    pub created_at: DateTime<Utc>,
}
