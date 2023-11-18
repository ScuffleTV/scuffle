use async_graphql::{Context, Subscription};
use futures_util::Stream;
use pb::ext::*;
use prost::Message;

use super::FollowStream;
use crate::api::auth::AuthError;
use crate::api::v1::gql::error::ext::*;
use crate::api::v1::gql::error::{GqlError, Result};
use crate::api::v1::gql::ext::ContextExt;
use crate::api::v1::gql::models::ulid::GqlUlid;
use crate::global::ApiGlobal;

pub struct ChannelSubscription<G: ApiGlobal>(std::marker::PhantomData<G>);

impl<G: ApiGlobal> Default for ChannelSubscription<G> {
	fn default() -> Self {
		Self(std::marker::PhantomData)
	}
}

#[Subscription]
impl<G: ApiGlobal> ChannelSubscription<G> {
	async fn channel_follows<'ctx>(
		&self,
		ctx: &'ctx Context<'ctx>,
		channel_id: GqlUlid,
	) -> Result<impl Stream<Item = Result<FollowStream>> + 'ctx> {
		let global = ctx.get_global::<G>();
		let request_context = ctx.get_req_context();

		let auth = request_context.auth().await?.ok_or(GqlError::Auth(AuthError::NotLoggedIn))?;

		// TODO: allow other users with permissions
		if auth.session.user_id.0 != channel_id.to_ulid() {
			return Err(GqlError::Unauthorized {
				field: "channel_follows",
			}
			.into());
		}

		let mut subscription = global
			.subscription_manager()
			.subscribe(format!("channel.{}.follows", channel_id.to_ulid().to_string()))
			.await
			.map_err_gql("failed to subscribe to channel follows")?;

		Ok(async_stream::stream!({
			while let Ok(message) = subscription.recv().await {
				let event = pb::scuffle::platform::internal::events::UserFollowChannel::decode(message.payload)
					.map_err_ignored_gql("failed to decode user follow")?;

				let user_id = event.user_id.to_ulid();
				let channel_id = event.channel_id.to_ulid();

				yield Ok(FollowStream {
					user_id: user_id.into(),
					channel_id: channel_id.into(),
					following: event.following,
				});
			}
		}))
	}

	async fn channel_followers_count<'ctx>(
		&self,
		ctx: &'ctx Context<'ctx>,
		channel_id: GqlUlid,
	) -> Result<impl Stream<Item = Result<i64>> + 'ctx> {
		let global = ctx.get_global::<G>();
		let request_context = ctx.get_req_context();

		let auth = request_context.auth().await?.ok_or(GqlError::Auth(AuthError::NotLoggedIn))?;

		// TODO: allow other users with permissions
		if auth.session.user_id.0 != channel_id.to_ulid() {
			return Err(GqlError::Unauthorized {
				field: "channel_followers_count",
			}
			.into());
		}

		let (mut followers,) = sqlx::query_as(
			r#"
			SELECT
				COUNT(*)
			FROM
				channel_user
			WHERE
				channel_id = $1
				AND following = true
			"#,
		)
		.bind(channel_id.to_uuid())
		.fetch_one(global.db().as_ref())
		.await?;

		let mut subscription = global
			.subscription_manager()
			.subscribe(format!("channel.{}.follows", channel_id.to_ulid().to_string()))
			.await
			.map_err_gql("failed to subscribe to channel follows")?;

		Ok(async_stream::stream!({
			yield Ok(followers);
			while let Ok(message) = subscription.recv().await {
				let event = pb::scuffle::platform::internal::events::UserFollowChannel::decode(message.payload)
					.map_err_ignored_gql("failed to decode user follow")?;

				if event.following {
					followers += 1;
				} else {
					followers -= 1;
				}

				yield Ok(followers);
			}
		}))
	}
}
