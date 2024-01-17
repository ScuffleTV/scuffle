use async_graphql::{Context, SimpleObject, Subscription};
use futures_util::Stream;
use pb::ext::*;
use prost::Message;
use ulid::Ulid;

use super::FollowStream;
use crate::api::auth::AuthError;
use crate::api::v1::gql::error::ext::*;
use crate::api::v1::gql::error::{GqlError, Result};
use crate::api::v1::gql::ext::ContextExt;
use crate::api::v1::gql::models::color::DisplayColor;
use crate::api::v1::gql::models::image_upload::ImageUpload;
use crate::api::v1::gql::models::ulid::GqlUlid;
use crate::global::ApiGlobal;
use crate::subscription::SubscriptionTopic;

pub struct UserSubscription<G: ApiGlobal>(std::marker::PhantomData<G>);

impl<G: ApiGlobal> Default for UserSubscription<G> {
	fn default() -> Self {
		Self(std::marker::PhantomData)
	}
}

#[derive(SimpleObject)]
struct UserDisplayNameStream {
	pub user_id: GqlUlid,
	pub display_name: String,
}

#[derive(SimpleObject)]
struct UserDisplayColorStream {
	pub user_id: GqlUlid,
	pub display_color: DisplayColor,
}

#[derive(SimpleObject)]
struct UserProfilePictureStream<G: ApiGlobal> {
	pub user_id: GqlUlid,
	pub profile_picture: Option<ImageUpload<G>>,
}

#[Subscription]
impl<G: ApiGlobal> UserSubscription<G> {
	async fn user_display_name<'ctx>(
		&self,
		ctx: &'ctx Context<'ctx>,
		user_id: GqlUlid,
	) -> Result<impl Stream<Item = Result<UserDisplayNameStream>> + 'ctx> {
		let global = ctx.get_global::<G>();

		let Some(display_name) = global
			.user_by_id_loader()
			.load(user_id.to_ulid())
			.await
			.map_err_ignored_gql("failed to fetch user")?
			.map(|u| u.display_name)
		else {
			return Err(GqlError::InvalidInput {
				fields: vec!["userId"],
				message: "user not found",
			}
			.into());
		};

		let mut subscription = global
			.subscription_manager()
			.subscribe(SubscriptionTopic::UserDisplayName(user_id.to_ulid()))
			.await
			.map_err_gql("failed to subscribe to user display name")?;

		Ok(async_stream::stream!({
			yield Ok(UserDisplayNameStream { user_id, display_name });

			while let Ok(message) = subscription.recv().await {
				let event = pb::scuffle::platform::internal::events::UserDisplayName::decode(message.payload)
					.map_err_ignored_gql("failed to decode user display name")?;

				let user_id = event.user_id.into_ulid();

				yield Ok(UserDisplayNameStream {
					user_id: user_id.into(),
					display_name: event.display_name,
				});
			}
		}))
	}

	async fn user_display_color<'ctx>(
		&self,
		ctx: &'ctx Context<'ctx>,
		user_id: GqlUlid,
	) -> Result<impl Stream<Item = Result<UserDisplayColorStream>> + 'ctx> {
		let global = ctx.get_global::<G>();

		let Some(display_color) = global
			.user_by_id_loader()
			.load(user_id.to_ulid())
			.await
			.map_err_ignored_gql("failed to fetch user")?
			.map(|u| u.display_color)
		else {
			return Err(GqlError::InvalidInput {
				fields: vec!["userId"],
				message: "user not found",
			}
			.into());
		};

		let mut subscription = global
			.subscription_manager()
			.subscribe(SubscriptionTopic::UserDisplayColor(user_id.to_ulid()))
			.await
			.map_err_gql("failed to subscribe to user display name")?;

		Ok(async_stream::stream!({
			yield Ok(UserDisplayColorStream {
				user_id,
				display_color: display_color.into(),
			});

			while let Ok(message) = subscription.recv().await {
				let event = pb::scuffle::platform::internal::events::UserDisplayColor::decode(message.payload)
					.map_err_ignored_gql("failed to decode user display name")?;

				let user_id = event.user_id.into_ulid();

				yield Ok(UserDisplayColorStream {
					user_id: user_id.into(),
					display_color: event.display_color.into(),
				});
			}
		}))
	}

	async fn user_profile_picture<'ctx>(
		&self,
		ctx: &'ctx Context<'ctx>,
		user_id: GqlUlid,
	) -> Result<impl Stream<Item = Result<UserProfilePictureStream<G>>> + 'ctx> {
		let global = ctx.get_global::<G>();

		let Some(profile_picture_id) = global
			.user_by_id_loader()
			.load(user_id.to_ulid())
			.await
			.map_err_ignored_gql("failed to fetch user")?
			.map(|u| u.profile_picture_id)
		else {
			return Err(GqlError::InvalidInput {
				fields: vec!["userId"],
				message: "user not found",
			}
			.into());
		};

		let mut subscription = global
			.subscription_manager()
			.subscribe(SubscriptionTopic::UserProfilePicture(user_id.to_ulid()))
			.await
			.map_err_gql("failed to subscribe to user display name")?;

		let profile_picture = if let Some(profile_picture_id) = profile_picture_id {
			global
				.uploaded_file_by_id_loader()
				.load(profile_picture_id)
				.await
				.map_err_ignored_gql("failed to fetch profile picture")?
				.map(ImageUpload::from_uploaded_file)
				.transpose()?
				.flatten()
		} else {
			None
		};

		Ok(async_stream::stream!({
			yield Ok(UserProfilePictureStream {
				user_id,
				profile_picture,
			});

			while let Ok(message) = subscription.recv().await {
				let event = pb::scuffle::platform::internal::events::UserProfilePicture::decode(message.payload)
					.map_err_ignored_gql("failed to decode user display name")?;

				let user_id = event.user_id.into_ulid();
				let profile_picture_id = event.profile_picture_id.map(|u| u.into_ulid());

				let profile_picture = if let Some(profile_picture_id) = profile_picture_id {
					global
						.uploaded_file_by_id_loader()
						.load(profile_picture_id)
						.await
						.map_err_ignored_gql("failed to fetch profile picture")?
						.map(ImageUpload::from_uploaded_file)
						.transpose()?
						.flatten()
				} else {
					None
				};

				yield Ok(UserProfilePictureStream {
					user_id: user_id.into(),
					profile_picture,
				});
			}
		}))
	}

	async fn user_following<'ctx>(
		&self,
		ctx: &'ctx Context<'ctx>,
		#[graphql(desc = "When specified, this subscription is limited to only this channel.")] channel_id: Option<GqlUlid>,
	) -> Result<impl Stream<Item = Result<FollowStream>> + 'ctx> {
		let global = ctx.get_global::<G>();
		let request_context = ctx.get_req_context();

		let auth = request_context
			.auth(global)
			.await?
			.ok_or(GqlError::Auth(AuthError::NotLoggedIn))?;

		let user_id: Ulid = auth.session.user_id;

		let mut subscription = global
			.subscription_manager()
			.subscribe(SubscriptionTopic::UserFollows(user_id))
			.await
			.map_err_gql("failed to subscribe to user follows")?;

		Ok(async_stream::stream!({
			if let Some(channel_id) = channel_id {
				let is_following = common::database::query(
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
				.bind(channel_id.to_ulid())
				.build_query_single_scalar::<bool>()
				.fetch_optional(global.db())
				.await
				.map_err_gql("failed to fetch channel_user")?
				.unwrap_or_default();

				yield Ok(FollowStream {
					user_id: user_id.into(),
					channel_id,
					following: is_following,
				});
			}

			while let Ok(message) = subscription.recv().await {
				let event = pb::scuffle::platform::internal::events::UserFollowChannel::decode(message.payload)
					.map_err_ignored_gql("failed to decode user follow")?;

				let user_id = event.user_id.into_ulid();

				let event_channel_id = event.channel_id.into_ulid();

				if channel_id.is_some_and(|i| event_channel_id != *i) {
					continue;
				}

				yield Ok(FollowStream {
					user_id: user_id.into(),
					channel_id: event_channel_id.into(),
					following: event.following,
				});
			}
		}))
	}
}
