use async_graphql::{ComplexObject, Context, SimpleObject};
use ulid::Ulid;

use crate::api::v1::gql::error::ResultExt;
use crate::api::v1::gql::{
    error::{GqlError, Result},
    ext::ContextExt,
};
use crate::database::{self, ChannelLink, RolePermission};

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
    pub links: Vec<ChannelLink>,
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

        let Some(category_id) = self.category_id else {
            return Ok(None);
        };

        let category = global
            .category_by_id_loader
            .load(category_id.into())
            .await
            .map_err_gql("failed to fetch category")?;

        Ok(category.map(Into::into))
    }

    async fn stream_key(&self, ctx: &Context<'_>) -> Result<&Option<String>> {
        let request_context = ctx.get_req_context();

        let auth = request_context.auth().await?;

        if let Some(auth) = auth {
            if Ulid::from(auth.session.user_id) == *self.id
                || auth.user_permissions.has_permission(RolePermission::Admin)
            {
                return Ok(&self.stream_key_);
            }
        }

        Err(GqlError::Unauthorized {
            field: "stream_key",
        }
        .into())
    }

    async fn followers_count(&self, ctx: &Context<'_>) -> Result<i64> {
        let global = ctx.get_global();

        let (followers,) = sqlx::query_as(
            "SELECT COUNT(*) FROM channel_user WHERE channel_id = $1 AND following = true",
        )
        .bind(self.id.to_uuid())
        .fetch_one(global.db.as_ref())
        .await
        .map_err_gql("failed to fetch followers")?;

        Ok(followers)
    }
}

impl From<database::Channel> for Channel {
    fn from(value: database::Channel) -> Self {
        let stream_key_ = value.get_stream_key();
        Self {
            id: value.id.0.into(),
            title: value.title,
            live_viewer_count: value.live_viewer_count,
            live_viewer_count_updated_at: value.live_viewer_count_updated_at.map(DateRFC3339),
            description: value.description,
            links: value.links.0,
            custom_thumbnail_id: value.custom_thumbnail_id.map(|v| Into::into(v.0)),
            offline_banner_id: value.offline_banner_id.map(|v| Into::into(v.0)),
            category_id: value.category_id.map(|v| Into::into(v.0)),
            last_live_at: value.last_live_at.map(DateRFC3339),
            stream_key_,
        }
    }
}
