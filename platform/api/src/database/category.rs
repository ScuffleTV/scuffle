use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct SearchResult {
    /// The category.
    #[sqlx(flatten)]
    pub category: Model,
    /// The similarity of the search query to the category's name.
    pub similarity: f64,
}

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct Model {
    pub id: Uuid,
    pub name: String,
    pub revision: i32,
    pub updated_at: DateTime<Utc>,
}
