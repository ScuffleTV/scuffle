use async_graphql::SimpleObject;

use super::ulid::GqlUlid;
use crate::database;

#[derive(SimpleObject, Clone)]
pub struct Role {
	pub id: GqlUlid,
	pub channel_id: Option<GqlUlid>,
	pub name: String,
	pub description: String,
	pub allowed_permissions: i64,
	pub denied_permissions: i64,
}

impl From<database::Role> for Role {
	fn from(value: database::Role) -> Self {
		Self {
			id: value.id.0.into(),
			channel_id: value.channel_id.map(|v| v.0.into()),
			name: value.name,
			description: value.description,
			allowed_permissions: value.allowed_permissions.bits(),
			denied_permissions: value.denied_permissions.bits(),
		}
	}
}
