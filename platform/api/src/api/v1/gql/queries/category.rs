use async_graphql::{Context, Object, SimpleObject};

use crate::api::v1::gql::error::ext::*;
use crate::api::v1::gql::error::Result;
use crate::api::v1::gql::ext::ContextExt;
use crate::api::v1::gql::models;
use crate::api::v1::gql::models::category::Category;
use crate::api::v1::gql::models::search_result::SearchResult;
use crate::api::v1::gql::models::ulid::GqlUlid;
use crate::database;
use crate::global::ApiGlobal;

pub struct CategoryQuery<G: ApiGlobal>(std::marker::PhantomData<G>);

impl<G: ApiGlobal> Default for CategoryQuery<G> {
	fn default() -> Self {
		Self(std::marker::PhantomData)
	}
}

#[derive(SimpleObject)]
struct CategorySearchResults {
	results: Vec<SearchResult<Category>>,
	total_count: u32,
}

impl From<Vec<database::SearchResult<database::Category>>> for CategorySearchResults {
	fn from(value: Vec<database::SearchResult<database::Category>>) -> Self {
		let total_count = value.first().map(|r| r.total_count).unwrap_or(0) as u32;
		Self {
			results: value.into_iter().map(Into::into).collect(),
			total_count,
		}
	}
}

#[Object]
impl<G: ApiGlobal> CategoryQuery<G> {
	async fn by_id(
		&self,
		ctx: &Context<'_>,
		#[graphql(desc = "The id of the category.")] id: GqlUlid,
	) -> Result<Option<models::category::Category>> {
		let global = ctx.get_global::<G>();

		let user = global
			.category_by_id_loader()
			.load(id.to_ulid())
			.await
			.map_err_ignored_gql("failed to fetch category")?;

		Ok(user.map(Into::into))
	}

	async fn search_by_name(
		&self,
		ctx: &Context<'_>,
		#[graphql(desc = "The search query.")] query: String,
		#[graphql(desc = "The result limit, default: 5", validator(minimum = 0, maximum = 50))] limit: Option<i32>,
		#[graphql(desc = "The result offset, default: 0", validator(minimum = 0, maximum = 950))] offset: Option<i32>,
	) -> Result<CategorySearchResults> {
		let global = ctx.get_global::<G>();

		let categories: Vec<database::SearchResult<database::Category>> = scuffle_utils::database::query("SELECT categories.*, similarity(name, $1), COUNT(*) OVER() AS total_count FROM categories WHERE name % $1 ORDER BY similarity DESC LIMIT $2 OFFSET $3")
			.bind(query)
			.bind(limit.unwrap_or(5))
			.bind(offset.unwrap_or(0))
			.build_query_as()
			.fetch_all(global.db())
			.await
			.map_err_gql("failed to search categories")?;

		Ok(categories.into())
	}
}
