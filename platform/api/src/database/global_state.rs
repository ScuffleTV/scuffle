use uuid::Uuid;

use super::role::Permission;

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct Model {
    pub role_order: Vec<Uuid>,
    pub default_permissions: Permission,
}
