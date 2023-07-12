use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
/// A grant of a global role to a user.
/// This allows for Admins to grant roles to other users.
/// See the `global_role` table for more information.
pub struct Model {
    /// The unique identifier for the grant.
    pub id: Uuid,
    /// Foreign key to the user table.
    pub user_id: Uuid,
    /// Foreign key to the global_role table.
    pub global_role_id: Uuid,
    /// The time the grant was created.
    pub created_at: DateTime<Utc>,
}
