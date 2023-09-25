use crate::api::v1::gql::error::Result;
use async_graphql::{Context, SimpleObject};

use super::{error::ResultExt, ext::ContextExt, models};

mod category;
mod user;

#[derive(Default, SimpleObject)]
#[graphql(complex)]
/// The root query type which contains root level fields.
pub struct Query {
    user: user::UserQuery,
    category: category::CategoryQuery,
}

#[derive(Clone, SimpleObject)]
struct SearchResults {
    users: Vec<models::user::UserSearchResult>,
    categories: Vec<models::category::CategorySearchResult>,
}

#[async_graphql::ComplexObject]
impl Query {
    async fn search(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The search query.")] query: String,
    ) -> Result<SearchResults> {
        let global = ctx.get_global();

        // TODO: perhaps this can be a single query, where we rank them together.
        let users = global
            .user_search_loader
            .load(query.clone())
            .await
            .ok()
            .flatten()
            .map_err_gql("failed to search users")?
            .into_iter()
            .map(Into::into)
            .collect();

        let categories = global
            .category_search_loader
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
