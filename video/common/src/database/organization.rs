use std::collections::HashMap;

use common::database::Ulid;

use super::DatabaseTable;

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct Organization {
	// The primary key for the organization
	pub id: Ulid,

	// The name of the organization
	pub name: String,

	// The date and time the organization was last updated
	pub updated_at: chrono::DateTime<chrono::Utc>,

	// Tags associated with the organization
	pub tags: sqlx::types::Json<HashMap<String, String>>,
}

impl DatabaseTable for Organization {
	const FRIENDLY_NAME: &'static str = "organization";
	const NAME: &'static str = "organizations";
}
