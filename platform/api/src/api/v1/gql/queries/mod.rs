use crate::{api::v1::gql::error::Result, global::ApiGlobal};
use async_graphql::{ComplexObject, Context, SimpleObject};

use super::{error::ResultExt, ext::ContextExt, models};

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

#[derive(Clone, SimpleObject)]
struct SearchResults<G: ApiGlobal> {
    users: Vec<models::user::UserSearchResult<G>>,
    categories: Vec<models::category::CategorySearchResult>,
}

#[ComplexObject]
impl<G: ApiGlobal> Query<G> {
    async fn search(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The search query.")] query: String,
    ) -> Result<SearchResults<G>> {
        let global = ctx.get_global::<G>();

        // TODO: perhaps this can be a single query, where we rank them together.
        let users = global
            .user_search_loader()
            .load(query.clone())
            .await
            .ok()
            .flatten()
            .map_err_gql("failed to search users")?
            .into_iter()
            .map(Into::into)
            .collect();

        let categories = global
            .category_search_loader()
            .load(query)
            .await
            .ok()
            .flatten()
            .map_err_gql("failed to search categories")?
            .into_iter()
            .map(Into::into)
            .collect();

        Ok(SearchResults { users, categories })
    }
}
