use async_graphql::{Context, SimpleObject, Subscription};
use futures_util::Stream;
use pb::ext::*;
use prost::Message;
use tokio_stream::StreamExt;

use super::FollowStream;
use crate::api::auth::AuthError;
use crate::api::v1::gql::error::ext::*;
use crate::api::v1::gql::error::{GqlError, Result};
use crate::api::v1::gql::ext::ContextExt;
use crate::api::v1::gql::models::ulid::GqlUlid;
use crate::global::ApiGlobal;
use crate::subscription::SubscriptionTopic;

pub struct ChannelSubscription<G: ApiGlobal>(std::marker::PhantomData<G>);

impl<G: ApiGlobal> Default for ChannelSubscription<G> {
	fn default() -> Self {
		Self(std::marker::PhantomData)
	}
}

#[derive(SimpleObject)]
struct ChannelTitleStream {
	pub channel_id: GqlUlid,
	pub title: Option<String>,
}

#[derive(SimpleObject)]
struct ChannelLiveStream {
	pub channel_id: GqlUlid,
	pub live: bool,
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

		let auth = request_context
			.auth(global)
			.await?
			.ok_or(GqlError::Auth(AuthError::NotLoggedIn))?;

		// TODO: allow other users with permissions
		if auth.session.user_id != channel_id.to_ulid() {
			return Err(GqlError::Unauthorized {
				field: "channel_follows",
			}
			.into());
		}

		let mut subscription = global
			.subscription_manager()
			.subscribe(SubscriptionTopic::ChannelFollows(channel_id.to_ulid()))
			.await
			.map_err_gql("failed to subscribe to channel follows")?;

		Ok(async_stream::stream!({
			while let Ok(message) = subscription.recv().await {
				let event = pb::scuffle::platform::internal::events::UserFollowChannel::decode(message.payload)
					.map_err_ignored_gql("failed to decode user follow")?;

				let user_id = event.user_id.into_ulid();
				let channel_id = event.channel_id.into_ulid();

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

		let stream = self.channel_follows(ctx, channel_id).await?;

		let mut followers = scuffle_utils::database::query(
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
		.bind(channel_id.to_ulid())
		.build_query_single_scalar()
		.fetch_one(global.db())
		.await?;

		Ok(stream.map(move |value| {
			let value = value?;

			if value.following {
				followers += 1;
			} else {
				followers -= 1;
			}

			Ok(followers)
		}))
	}

	async fn channel_title<'ctx>(
		&self,
		ctx: &'ctx Context<'ctx>,
		channel_id: GqlUlid,
	) -> Result<impl Stream<Item = Result<ChannelTitleStream>> + 'ctx> {
		let global = ctx.get_global::<G>();

		let Some(title) = global
			.user_by_id_loader()
			.load(channel_id.to_ulid())
			.await
			.map_err_ignored_gql("failed to fetch user")?
			.map(|u| u.channel.title)
		else {
			return Err(GqlError::InvalidInput {
				fields: vec!["channelId"],
				message: "channel not found",
			}
			.into());
		};

		let mut subscription = global
			.subscription_manager()
			.subscribe(SubscriptionTopic::ChannelTitle(channel_id.to_ulid()))
			.await
			.map_err_gql("failed to subscribe to channel title")?;

		Ok(async_stream::stream!({
			yield Ok(ChannelTitleStream { channel_id, title });

			while let Ok(message) = subscription.recv().await {
				let event = pb::scuffle::platform::internal::events::ChannelTitle::decode(message.payload)
					.map_err_ignored_gql("failed to decode channel title event")?;

				let channel_id = event.channel_id.into_ulid();

				yield Ok(ChannelTitleStream {
					channel_id: channel_id.into(),
					title: Some(event.title),
				});
			}
		}))
	}

	async fn channel_live<'ctx>(
		&self,
		ctx: &'ctx Context<'ctx>,
		channel_id: GqlUlid,
	) -> Result<impl Stream<Item = Result<ChannelLiveStream>> + 'ctx> {
		let global = ctx.get_global::<G>();

		let mut subscription = global
			.subscription_manager()
			.subscribe(SubscriptionTopic::ChannelLive(channel_id.to_ulid()))
			.await
			.map_err_gql("failed to subscribe to channel live")?;

		Ok(async_stream::stream!({
			while let Ok(message) = subscription.recv().await {
				let event = pb::scuffle::platform::internal::events::ChannelLive::decode(message.payload)
					.map_err_ignored_gql("failed to decode channel live event")?;

				yield Ok(ChannelLiveStream {
					channel_id: event.channel_id.into_ulid().into(),
					live: event.live,
				});
			}
		}))
	}
}
