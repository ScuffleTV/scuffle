use async_graphql::{Context, SimpleObject, Subscription};
use fred::types::RedisValue;
use futures_util::Stream;
use prost::Message;
use uuid::Uuid;

use crate::{
    api::v1::gql::{
        error::{GqlError, Result, ResultExt},
        ext::ContextExt,
    },
    pb,
};

#[derive(Default)]
pub struct UserSubscription;

#[derive(SimpleObject)]
struct DisplayNameStream {
    pub username: String,
    pub display_name: String,
}

#[Subscription]
impl UserSubscription {
    async fn user_display_name<'ctx>(
        &self,
        ctx: &'ctx Context<'ctx>,
        user_id: Uuid,
    ) -> Result<impl Stream<Item = Result<DisplayNameStream>> + 'ctx> {
        let global = ctx.get_global();

        let Some(mut user) = global.user_by_id_loader.load_one(user_id).await.map_err_gql("failed to fetch user")? else {
            return Err(GqlError::NotFound.with_message("user not found").with_field(vec!["user_id"]));
        };

        let mut subscription = global
            .subscription_manager
            .subscribe(format!("user.{}.display_name", user_id))
            .await
            .map_err_gql("failed to subscribe to user display name")?;

        Ok(async_stream::stream! {
            yield Ok(DisplayNameStream {
                display_name: user.display_name.clone(),
                username: user.username.clone(),
            });

            while let Ok(value) = subscription.recv().await {
                let value = match value {
                    RedisValue::String(val) => val,
                    _ => {
                        yield Err(GqlError::InternalServerError.with_message("invalid redis value type"));
                        break;
                    },
                }.into_inner();

                let event = pb::scuffle::types::api::UserDisplayName::decode(value).map_err_gql("failed to decode user display name")?;

                if let Some(username) = event.username {
                    user.username = username;
                }

                if let Some(display_name) = event.display_name {
                    user.display_name = display_name;
                }

                yield Ok(DisplayNameStream {
                    display_name: user.display_name.clone(),
                    username: user.username.clone(),
                });
            }
        })
    }
}
