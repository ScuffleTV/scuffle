use async_graphql::SimpleObject;

use crate::database::global_role;
use uuid::Uuid;

use super::date::DateRFC3339;

#[derive(SimpleObject, Clone)]
pub struct GlobalRole {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub rank: i32,
    pub allowed_permissions: i64,
    pub denied_permissions: i64,
    pub created_at: DateRFC3339,
}

impl From<global_role::Model> for GlobalRole {
    fn from(value: global_role::Model) -> Self {
        Self {
            id: value.id,
            created_at: value.created_at.into(),
            name: value.name,
            description: value.description,
            rank: value.rank as i32,
            allowed_permissions: value.allowed_permissions.bits(),
            denied_permissions: value.denied_permissions.bits(),
        }
    }
}
