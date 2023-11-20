use async_graphql::{ComplexObject, Context, SimpleObject};
use common::database::Ulid;
use sqlx::FromRow;

use super::error::ext::*;
use super::ext::ContextExt;
use super::models::search_result::{SearchAllResultData, SearchAllResults, SearchResult};
use crate::api::v1::gql::error::Result;
use crate::global::ApiGlobal;

mod category;
mod user;

#[derive(SimpleObject)]
#[graphql(complex)]
/// The root query type which contains root level fields.
pub struct Query<G: ApiGlobal> {
	pub category: category::CategoryQuery<G>,
	pub user: user::UserQuery<G>,
}

impl<G: ApiGlobal> Default for Query<G> {
	fn default() -> Self {
		Self {
			category: Default::default(),
			user: Default::default(),
		}
	}
}

#[derive(FromRow)]
struct SearchResultQueryResponse {
	r#type: i64,
	id: Ulid,
	similarity: f64,
	total_count: i64,
}

#[ComplexObject]
impl<G: ApiGlobal> Query<G> {
	async fn search(
		&self,
		ctx: &Context<'_>,
		#[graphql(desc = "The search query.")] query: String,
		#[graphql(desc = "The result limit, default: 5", validator(minimum = 0, maximum = 50))] limit: Option<i32>,
		#[graphql(desc = "The result offset, default: 0", validator(minimum = 0, maximum = 950))] offset: Option<i32>,
	) -> Result<SearchAllResults<G>> {
		let global = ctx.get_global::<G>();

		let query_results: Vec<SearchResultQueryResponse> = sqlx::query_as(
			r#"
			WITH CombinedResults AS (
				SELECT
					0 as type,
					id,
					similarity(username, $1),
					COUNT(*) OVER() AS total_count
				FROM
					users
				WHERE
					username % $1

				UNION

				SELECT
					1 as type,
					id,
					similarity(name, $1),
					COUNT(*) OVER() AS total_count
				FROM
					categories
				WHERE
					name % $1
			)
			SELECT
				*,
				COUNT(*) OVER() AS total_count
			FROM
				CombinedResults
			ORDER BY similarity DESC LIMIT $2 OFFSET $3;
			"#,
		)
		.bind(query)
		.bind(limit.unwrap_or(5))
		.bind(offset.unwrap_or(0))
		.fetch_all(global.db().as_ref())
		.await
		.map_err_gql("failed to search")?;

		let total_count = query_results.first().map(|r| r.total_count).unwrap_or(0) as u32;

		let (users, categories) = query_results.iter().fold((Vec::new(), Vec::new()), |mut store, item| {
			match item.r#type {
				0 => &mut store.0,
				1 => &mut store.1,
				_ => unreachable!(),
			}
			.push(item.id.0);

			store
		});

		let (users, categories) = tokio::try_join!(
			global.user_by_id_loader().load_many(users.into_iter()),
			global.category_by_id_loader().load_many(categories.into_iter()),
		)
		.map_err_ignored_gql("failed to fetch users and categories")?;

		let results = query_results
			.iter()
			.filter_map(|r| {
				let object = match r.r#type {
					0 => SearchAllResultData::User(Box::new(users.get(&r.id.0)?.clone().into())),
					1 => SearchAllResultData::Category(categories.get(&r.id.0)?.clone().into()),
					_ => unreachable!(),
				};

				Some(SearchResult {
					object,
					similarity: r.similarity,
				})
			})
			.collect();

		Ok(SearchAllResults { results, total_count })
	}
}
