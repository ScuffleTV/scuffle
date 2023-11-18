use common::database::Ulid;

use super::RolePermission;

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct GlobalState {
	pub role_order: Vec<Ulid>,
	pub default_permissions: RolePermission,
}
