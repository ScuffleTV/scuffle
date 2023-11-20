use async_graphql::{OutputType, SimpleObject, Union};

use super::category::Category;
use super::user::User;
use crate::database;
use crate::global::ApiGlobal;

#[derive(SimpleObject)]
#[graphql(concrete(name = "UserSearchResult", params("User<G>"), bounds("G: ApiGlobal")))]
#[graphql(concrete(name = "CategorySearchResult", params(Category)))]
#[graphql(concrete(name = "SearchResult", params("SearchAllResultData<G>"), bounds("G: ApiGlobal")))]
pub struct SearchResult<T: OutputType> {
	pub object: T,
	pub similarity: f64,
}

impl<T: Clone + std::fmt::Debug + Into<O>, O: OutputType> From<database::SearchResult<T>> for SearchResult<O> {
	fn from(value: database::SearchResult<T>) -> Self {
		Self {
			object: value.object.into(),
			similarity: value.similarity,
		}
	}
}

#[derive(SimpleObject)]
pub struct SearchAllResults<G: ApiGlobal> {
	pub results: Vec<SearchResult<SearchAllResultData<G>>>,
	pub total_count: u32,
}

#[derive(Union)]
pub enum SearchAllResultData<G: ApiGlobal> {
	Category(Category),
	User(Box<User<G>>),
}
