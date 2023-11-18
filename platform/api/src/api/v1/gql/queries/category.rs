use async_graphql::{Context, Object};

use crate::api::v1::gql::error::ext::*;
use crate::api::v1::gql::error::Result;
use crate::api::v1::gql::ext::ContextExt;
use crate::api::v1::gql::models::ulid::GqlUlid;
use crate::api::v1::gql::models::{self};
use crate::global::ApiGlobal;

pub struct CategoryQuery<G: ApiGlobal>(std::marker::PhantomData<G>);

impl<G: ApiGlobal> Default for CategoryQuery<G> {
	fn default() -> Self {
		Self(std::marker::PhantomData)
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
	) -> Result<Vec<models::category::CategorySearchResult>> {
		let global = ctx.get_global::<G>();

		let categories = global
			.category_search_loader()
			.load(query)
			.await
			.map_err_ignored_gql("failed to fetch categories")?
			.map_err_gql("failed to fetch categories")?;

		Ok(categories.into_iter().map(Into::into).collect())
	}
}
