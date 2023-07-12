use bitmask_enum::bitmask;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
/// A role that can be granted to a user in a channel.
/// Roles can allow or deny permissions to a user.
/// The rank indicates the order in which the role permissions are applied.
/// Roles can have many users granted to them. See the `channel_role_grant` table for more information.
pub struct Model {
    /// The unique identifier for the role.
    pub id: Uuid,
    /// Foreign key to the users table.
    pub channel_id: Uuid,
    /// The name of the role.
    pub name: String,
    /// The description of the role.
    pub description: String,
    /// The rank of the role. (higher rank = priority) unique per channel (-1 is default role)
    pub rank: i32,
    /// The permissions granted by this role.
    pub allowed_permissions: Permission,
    /// The permissions denied by this role.
    pub denied_permissions: Permission,
    /// The time the role was created.
    pub created_at: DateTime<Utc>,
}

#[bitmask(i64)]
pub enum Permission {}

impl Default for Permission {
    fn default() -> Self {
        Self::none()
    }
}
