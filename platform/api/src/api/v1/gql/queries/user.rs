use async_graphql::{Context, Object, SimpleObject};

use crate::api::auth::AuthError;
use crate::api::v1::gql::error::ext::*;
use crate::api::v1::gql::error::{GqlError, Result};
use crate::api::v1::gql::ext::ContextExt;
use crate::api::v1::gql::models;
use crate::api::v1::gql::models::search_result::SearchResult;
use crate::api::v1::gql::models::ulid::GqlUlid;
use crate::api::v1::gql::models::user::User;
use crate::database;
use crate::global::ApiGlobal;

/// All user queries
pub struct UserQuery<G: ApiGlobal>(std::marker::PhantomData<G>);

impl<G: ApiGlobal> Default for UserQuery<G> {
	fn default() -> Self {
		Self(std::marker::PhantomData)
	}
}

#[derive(SimpleObject)]
struct UserSearchResults<G: ApiGlobal> {
	results: Vec<SearchResult<User<G>>>,
	total_count: u32,
}

impl<G: ApiGlobal> From<Vec<database::SearchResult<database::User>>> for UserSearchResults<G> {
	fn from(value: Vec<database::SearchResult<database::User>>) -> Self {
		let total_count = value.first().map(|r| r.total_count).unwrap_or(0) as u32;
		Self {
			results: value.into_iter().map(Into::into).collect(),
			total_count,
		}
	}
}

#[Object]
impl<G: ApiGlobal> UserQuery<G> {
	/// Get the user of the current context(session)
	async fn with_current_context(&self, ctx: &Context<'_>) -> Result<models::user::User<G>> {
		let global = ctx.get_global::<G>();
		let auth = ctx
			.get_req_context()
			.auth(global)
			.await?
			.ok_or(GqlError::Auth(AuthError::NotLoggedIn))?;

		global
			.user_by_id_loader()
			.load(auth.session.user_id.0)
			.await
			.map_err_ignored_gql("failed to fetch user")?
			.map_err_gql(GqlError::NotFound("user"))
			.map(Into::into)
	}

	/// Get a user by their username
	async fn by_username(
		&self,
		ctx: &Context<'_>,
		#[graphql(desc = "The username of the user.")] username: String,
	) -> Result<Option<models::user::User<G>>> {
		let global = ctx.get_global::<G>();

		let user = global
			.user_by_username_loader()
			.load(username.to_lowercase())
			.await
			.map_err_ignored_gql("failed to fetch user")?;

		Ok(user.map(Into::into))
	}

	/// Get a user by their id
	async fn by_id(
		&self,
		ctx: &Context<'_>,
		#[graphql(desc = "The id of the user.")] id: GqlUlid,
	) -> Result<Option<models::user::User<G>>> {
		let global = ctx.get_global::<G>();

		let user = global
			.user_by_id_loader()
			.load(id.to_ulid())
			.await
			.map_err_ignored_gql("failed to fetch user")?;

		Ok(user.map(models::user::User::from))
	}

	async fn search_by_username(
		&self,
		ctx: &Context<'_>,
		#[graphql(desc = "The search query.")] query: String,
		#[graphql(desc = "The result limit, default: 5", validator(minimum = 0, maximum = 50))] limit: Option<i32>,
		#[graphql(desc = "The result offset, default: 0", validator(minimum = 0, maximum = 950))] offset: Option<i32>,
	) -> Result<UserSearchResults<G>> {
		let global = ctx.get_global::<G>();

		let users: Vec<database::SearchResult<database::User>> = sqlx::query_as("SELECT users.*, similarity(username, $1), COUNT(*) OVER() AS total_count FROM users WHERE username % $1 ORDER BY similarity DESC LIMIT $2 OFFSET $3")
			.bind(query)
			.bind(limit.unwrap_or(5))
			.bind(offset.unwrap_or(0))
			.fetch_all(global.db().as_ref())
			.await
			.map_err_gql("failed to search users")?;

		Ok(users.into())
	}

	/// Get if the current user is following a given channel
	async fn is_following(&self, ctx: &Context<'_>, channel_id: GqlUlid) -> Result<bool> {
		let global = ctx.get_global::<G>();
		let request_context = ctx.get_req_context();

		let auth = request_context
			.auth(global)
			.await?
			.ok_or(GqlError::Auth(AuthError::NotLoggedIn))?;

		let (is_following,): (bool,) = sqlx::query_as(
			r#"
			SELECT
				following
			FROM
				channel_user
			WHERE
				user_id = $1
				AND channel_id = $2
			"#,
		)
		.bind(auth.session.user_id)
		.bind(channel_id.to_uuid())
		.fetch_optional(global.db().as_ref())
		.await?
		.unwrap_or((false,));

		Ok(is_following)
	}

	async fn following(
		&self,
		ctx: &Context<'_>,
		#[graphql(desc = "The id of the user.")] id: GqlUlid,
		#[graphql(desc = "Restricts the number of returned users, default: no limit")] limit: Option<u32>,
	) -> Result<Vec<models::user::User<G>>> {
		let global = ctx.get_global::<G>();
		let request_context = ctx.get_req_context();

		let auth = request_context
			.auth(global)
			.await?
			.ok_or(GqlError::Auth(AuthError::NotLoggedIn))?;

		// TODO: Also allow users with permission
		if id.to_ulid() != auth.session.user_id.0 {
			return Err(GqlError::Unauthorized { field: "following" }.into());
		}

		// This query is not very good, we should have some paging mechinsm with ids.
		let channels: Vec<database::User> = sqlx::query_as(
			r#"
			SELECT
				users.*
			FROM
				channel_user
			INNER JOIN
				users
			ON
				users.id = channel_user.channel_id
			WHERE
				channel_user.user_id = $1
				AND channel_user.following = true
			ORDER BY
				users.channel_live_viewer_count DESC,
				users.channel_last_live_at DESC
			LIMIT $2
			"#,
		)
		.bind(id.to_uuid())
		.bind(limit.map(|l| l as i64))
		.fetch_all(global.db().as_ref())
		.await?;

		Ok(channels.into_iter().map(Into::into).collect())
	}
}
