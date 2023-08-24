use bitmask_enum::bitmask;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Default, sqlx::FromRow)]
/// A role that can be granted to a user globally.
/// Roles can allow or deny permissions to a user.
/// The rank indicates the order in which the role permissions are applied.
/// Roles can have many users granted to them. See the `global_role_grant` table for more information.
pub struct Model {
    /// The unique identifier for the role.
    pub id: Uuid,
    /// The name of the role.
    pub name: String,
    /// The description of the role.
    pub description: String,
    /// The rank of the role. (higher rank = priority)  (-1 is default role)
    pub rank: i64,
    /// The permissions granted by this role.
    // pub allowed_permissions: Permission,
    /// The permissions denied by this role.
    // pub denied_permissions: Permission,
    /// The time the role was created.
    pub created_at: DateTime<Utc>,
}

#[bitmask(i64)]
pub enum Permission {
    /// Can do anything
    Admin,
    /// Can start streaming
    GoLive,
    /// Has access to transcoded streams
    StreamTranscoding,
    /// Has access to recorded streams
    StreamRecording,
}

impl Default for Permission {
    fn default() -> Self {
        Self::none()
    }
}

impl Permission {
    /// Checks if the current permission set has the given permission.
    /// Admin permissions always return true. Otherwise, the permission is checked against the current permission set.
    pub fn has_permission(&self, other: Self) -> bool {
        (*self & Self::Admin == Self::Admin) || (*self & other == other)
    }
}
