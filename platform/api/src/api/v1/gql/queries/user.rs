use async_graphql::{Context, Object};

use crate::{
    api::v1::gql::{
        error::{GqlError, Result, ResultExt},
        ext::ContextExt,
        models::{self, ulid::GqlUlid},
    },
    database,
};

#[derive(Default)]
/// All user queries
pub struct UserQuery;

#[Object]
impl UserQuery {
    /// Get a user by their username
    async fn by_username(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The username of the user.")] username: String,
    ) -> Result<Option<models::user::User>> {
        let global = ctx.get_global();

        let user = global
            .user_by_username_loader
            .load(username.to_lowercase())
            .await
            .ok()
            .map_err_gql("failed to fetch user")?;

        Ok(user.map(Into::into))
    }

    /// Get a user by their id
    async fn by_id(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The id of the user.")] id: GqlUlid,
    ) -> Result<Option<models::user::User>> {
        let global = ctx.get_global();

        let user = global
            .user_by_id_loader
            .load(id.to_ulid())
            .await
            .ok()
            .map_err_gql("failed to fetch user")?;

        Ok(user.map(models::user::User::from))
    }

    async fn search_by_username(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The search query.")] query: String,
    ) -> Result<Vec<models::user::UserSearchResult>> {
        let global = ctx.get_global();

        let users = global
            .user_search_loader
            .load(query)
            .await
            .ok()
            .flatten()
            .map_err_gql("failed to search users")?;

        Ok(users.into_iter().map(Into::into).collect())
    }

    /// Get if the current user is following a given channel
    async fn is_following(&self, ctx: &Context<'_>, channel_id: GqlUlid) -> Result<bool> {
        let global = ctx.get_global();
        let request_context = ctx.get_req_context();

        let auth = request_context
            .auth()
            .await
            .ok_or(GqlError::Unauthorized.with_message("You need to be logged in"))?;

        let (is_following,): (bool,) = sqlx::query_as(
            "SELECT following FROM channel_user WHERE user_id = $1 AND channel_id = $2",
        )
        .bind(auth.session.user_id)
        .bind(channel_id.to_uuid())
        .fetch_optional(global.db.as_ref())
        .await
        .map_err_gql("Failed to fetch channel_user")?
        .unwrap_or((false,));

        Ok(is_following)
    }

    async fn following(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The id of the user.")] id: GqlUlid,
        #[graphql(desc = "Restricts the number of returned users, default: no limit")]
        limit: Option<u32>,
    ) -> Result<Vec<models::user::User>> {
        let global = ctx.get_global();
        let request_context = ctx.get_req_context();

        let auth = request_context
            .auth()
            .await
            .ok_or(GqlError::Unauthorized.with_message("You need to be logged in"))?;

        // TODO: Also allow users with permission
        if id.to_ulid() != auth.session.user_id.0 {
            return Err(GqlError::Unauthorized.with_message("You can only fetch your own follows"));
        }

        // This query is not very good, we should have some paging mechinsm with ids.
        let channels: Vec<database::User> = sqlx::query_as(
            "SELECT users.* FROM channel_user INNER JOIN users ON users.id = channel_user.channel_id WHERE channel_user.user_id = $1 AND channel_user.following = true ORDER BY users.channel_live_viewer_count DESC, users.channel_last_live_at DESC LIMIT $2",
        )
        .bind(id.to_uuid())
        .bind(limit.map(|l| l as i64))
        .fetch_all(global.db.as_ref())
        .await
        .map_err_gql("Failed to fetch channels")?;

        Ok(channels.into_iter().map(Into::into).collect())
    }
}
