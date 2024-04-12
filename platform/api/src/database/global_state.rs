use ulid::Ulid;

use super::RolePermission;

#[derive(Debug, Clone, Default, postgres_from_row::FromRow)]
pub struct GlobalState {
	pub role_order: Vec<Ulid>,
	pub default_permissions: RolePermission,
}
