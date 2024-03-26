#[derive(Debug, Clone, Default, postgres_from_row::FromRow)]
pub struct SearchResult<T: Clone + std::fmt::Debug> {
	/// The category.
	#[from_row(flatten)]
	pub object: T,
	/// The similarity of the search query to the category's name.
	pub similarity: f64,
	/// The total count of results (ignoring the limit)
	pub total_count: i32,
}
