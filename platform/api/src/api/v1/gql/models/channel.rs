use async_graphql::{ComplexObject, Context, SimpleObject};
use ulid::Ulid;
use uuid::Uuid;

use crate::api::v1::gql::error::ResultExt;
use crate::database::role;
use crate::{
    api::v1::gql::{
        error::{GqlError, Result},
        ext::ContextExt,
    },
    database::channel,
};

use super::category::Category;
use super::{date::DateRFC3339, ulid::GqlUlid};

#[derive(SimpleObject, Clone)]
#[graphql(complex)]
pub struct Channel {
    pub id: GqlUlid,
    pub title: Option<String>,
    pub live_viewer_count: Option<i32>,
    pub live_viewer_count_updated_at: Option<DateRFC3339>,
    pub description: Option<String>,
    pub links: Vec<channel::Link>,
    pub custom_thumbnail_id: Option<GqlUlid>,
    pub offline_banner_id: Option<GqlUlid>,
    pub category_id: Option<GqlUlid>,
    pub last_live_at: Option<DateRFC3339>,

    // Private fields
    #[graphql(skip)]
    pub stream_key_: Option<String>,
}

#[ComplexObject]
impl Channel {
    async fn category(&self, ctx: &Context<'_>) -> Result<Option<Category>> {
        let global = ctx.get_global();

        let Some(category_id) = &self.category_id else {
            return Ok(None);
        };

        let category = global
            .category_by_id_loader
            .load_one(Into::<Uuid>::into(*category_id))
            .await
            .map_err_gql("Failed to fetch category")?;

        Ok(category.map(Into::into))
    }

    async fn stream_key(&self, ctx: &Context<'_>) -> Result<&Option<String>> {
        let request_context = ctx.get_req_context();

        let auth = request_context.auth().await;

        if let Some(auth) = auth {
            if Ulid::from(auth.session.user_id) == *self.id
                || auth
                    .user_permissions
                    .has_permission(role::Permission::Admin)
            {
                return Ok(&self.stream_key_);
            }
        }

        Err(GqlError::Unauthorized
            .with_message("you are not allowed to see this field")
            .with_field(vec!["stream_key"]))
    }

    async fn followers_count(&self, ctx: &Context<'_>) -> Result<i64> {
        let global = ctx.get_global();

        let (followers,) = sqlx::query_as(
            "SELECT COUNT(*) FROM channel_user WHERE channel_id = $1 AND following = true",
        )
        .bind(Into::<Uuid>::into(self.id))
        .fetch_one(&*global.db)
        .await
        .map_err_gql("Failed to fetch followers")?;

        Ok(followers)
    }
}

impl From<channel::Model> for Channel {
    fn from(value: channel::Model) -> Self {
        let stream_key_ = value.get_stream_key();
        Self {
            id: value.id.into(),
            title: value.title,
            live_viewer_count: value.live_viewer_count,
            live_viewer_count_updated_at: value.live_viewer_count_updated_at.map(DateRFC3339),
            description: value.description,
            links: value.links.0,
            custom_thumbnail_id: value.custom_thumbnail_id.map(Into::into),
            offline_banner_id: value.offline_banner_id.map(Into::into),
            category_id: value.category_id.map(Into::into),
            last_live_at: value.last_live_at.map(DateRFC3339),
            stream_key_,
        }
    }
}
