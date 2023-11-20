use async_graphql::SimpleObject;

use super::date::DateRFC3339;
use super::ulid::GqlUlid;
use crate::database;

#[derive(SimpleObject, Clone)]
pub struct Category {
	pub id: GqlUlid,
	pub name: String,
	pub revision: i32,
	pub updated_at: DateRFC3339,
}

impl From<database::Category> for Category {
	fn from(value: database::Category) -> Self {
		Self {
			id: value.id.0.into(),
			name: value.name,
			revision: value.revision,
			updated_at: value.updated_at.into(),
		}
	}
}
