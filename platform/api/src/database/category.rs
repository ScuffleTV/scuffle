use chrono::{DateTime, Utc};
use ulid::Ulid;

#[derive(Debug, Clone, Default, postgres_from_row::FromRow)]
pub struct Category {
	pub id: Ulid,
	pub name: String,
	pub revision: i32,
	pub updated_at: DateTime<Utc>,
}
