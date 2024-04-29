use async_graphql::{Context, Object};
use prost::Message;

use crate::api::auth::AuthError;
use crate::api::v1::gql::error::ext::{OptionExt, ResultExt};
use crate::api::v1::gql::error::{GqlError, Result};
use crate::api::v1::gql::ext::ContextExt;
use crate::api::v1::gql::models::user::User;
use crate::database;
use crate::global::ApiGlobal;
use crate::subscription::SubscriptionTopic;

pub struct ChannelMutation<G>(std::marker::PhantomData<G>);

impl<G: ApiGlobal> Default for ChannelMutation<G> {
	fn default() -> Self {
		Self(std::marker::PhantomData)
	}
}

#[Object]
impl<G: ApiGlobal> ChannelMutation<G> {
	async fn title(&self, ctx: &Context<'_>, #[graphql(desc = "The new title.")] title: String) -> Result<User<G>> {
		let global = ctx.get_global::<G>();
		let request_context = ctx.get_req_context();

		let auth = request_context
			.auth(global)
			.await?
			.map_err_gql(GqlError::Auth(AuthError::NotLoggedIn))?;

		let user: database::User = scuffle_utils::database::query(
			r#"
			UPDATE users
			SET
				channel_title = $1,
				updated_at = NOW()
			WHERE
				id = $2
			RETURNING *
			"#,
		)
		.bind(title.clone())
		.bind(auth.session.user_id)
		.build_query_as()
		.fetch_one(global.db())
		.await?;

		global
			.nats()
			.publish(
				SubscriptionTopic::ChannelTitle(user.id),
				pb::scuffle::platform::internal::events::ChannelTitle {
					channel_id: Some(user.id.into()),
					title,
				}
				.encode_to_vec()
				.into(),
			)
			.await
			.map_err_gql("failed to publish channel title")?;

		Ok(user.into())
	}
}
