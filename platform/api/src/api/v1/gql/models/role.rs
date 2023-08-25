use async_graphql::SimpleObject;

use crate::database::role;

use super::ulid::GqlUlid;

#[derive(SimpleObject, Clone)]
pub struct Role {
    pub id: GqlUlid,
    pub channel_id: Option<GqlUlid>,
    pub name: String,
    pub description: String,
    pub allowed_permissions: i64,
    pub denied_permissions: i64,
}

impl From<role::Model> for Role {
    fn from(value: role::Model) -> Self {
        Self {
            id: value.id.into(),
            channel_id: value.channel_id.map(Into::into),
            name: value.name,
            description: value.description,
            allowed_permissions: value.allowed_permissions.bits(),
            denied_permissions: value.denied_permissions.bits(),
        }
    }
}
