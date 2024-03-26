use chrono::{DateTime, Utc};
use ulid::Ulid;

#[derive(Debug, Clone, Default, postgres_from_row::FromRow)]
pub struct Category {
	pub id: Ulid,
	pub igdb_id: Option<i32>,
	pub name: String,
	pub aliases: Vec<String>,
	pub keywords: Vec<String>,
	pub storyline: Option<String>,
	pub summary: Option<String>,
	pub over_18: bool,
	pub cover_id: Option<Ulid>,
	pub rating: f64,
	pub artwork_ids: Vec<Ulid>,
	pub igdb_similar_game_ids: Vec<i32>,
	pub websites: Vec<String>,
	pub updated_at: DateTime<Utc>,
}
