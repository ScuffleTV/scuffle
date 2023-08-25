use async_graphql::{Context, Object};
use uuid::Uuid;

use crate::api::v1::gql::{
    error::{Result, ResultExt},
    ext::ContextExt,
    models::{self, ulid::GqlUlid},
};

#[derive(Default)]
pub struct CategoryQuery;

#[Object]
impl CategoryQuery {
    async fn by_id(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The id of the category.")] id: GqlUlid,
    ) -> Result<Option<models::category::Category>> {
        let global = ctx.get_global();

        let user = global
            .category_by_id_loader
            .load_one(Into::<Uuid>::into(id))
            .await
            .map_err_gql("failed to fetch category")?;

        Ok(user.map(Into::into))
    }

    async fn search_by_name(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The search query.")] query: String,
    ) -> Result<Vec<models::category::CategorySearchResult>> {
        let global = ctx.get_global();

        let categories = global
            .category_search_loader
            .load_one(query)
            .await
            .ok()
            .flatten()
            .map_err_gql("failed to search categories")?;

        Ok(categories.into_iter().map(Into::into).collect())
    }
}
