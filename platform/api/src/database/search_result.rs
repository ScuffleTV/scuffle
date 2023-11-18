#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct SearchResult<T: Clone + std::fmt::Debug> {
	/// The category.
	#[sqlx(flatten)]
	pub object: T,
	/// The similarity of the search query to the category's name.
	pub similarity: f64,
}
