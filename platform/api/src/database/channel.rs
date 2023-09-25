use async_graphql::SimpleObject;
use chrono::{DateTime, Utc};

use super::Ulid;

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct Channel {
    /// Ulid of the channel
    pub id: Ulid,
    /// The current stream's title
    #[sqlx(rename = "channel_title")]
    pub title: Option<String>,
    /// The current stream's live viewer count
    #[sqlx(rename = "channel_live_viewer_count")]
    pub live_viewer_count: Option<i32>,
    /// The time the current stream's live viewer count was last updated
    #[sqlx(rename = "channel_live_viewer_count_updated_at")]
    pub live_viewer_count_updated_at: Option<DateTime<Utc>>,
    /// The current stream's description
    #[sqlx(rename = "channel_description")]
    pub description: Option<String>,
    /// The social links
    #[sqlx(rename = "channel_links")]
    pub links: sqlx::types::Json<Vec<ChannelLink>>,
    /// The current stream's thumbnail
    #[sqlx(rename = "channel_custom_thumbnail_id")]
    pub custom_thumbnail_id: Option<Ulid>,
    /// The offline banner of the channel
    #[sqlx(rename = "channel_offline_banner_id")]
    pub offline_banner_id: Option<Ulid>,
    /// The current stream's category
    #[sqlx(rename = "channel_category_id")]
    pub category_id: Option<Ulid>,
    /// Channel stream key
    #[sqlx(rename = "channel_stream_key")]
    pub stream_key: Option<String>,
    /// Channel roles order
    #[sqlx(rename = "channel_role_order")]
    pub role_order: Vec<Ulid>,
    /// Channel default permissions
    #[sqlx(rename = "channel_default_permissions")]
    pub default_permissions: i64,
    /// Channel permissions for followers
    #[sqlx(rename = "channel_following_permission")]
    pub following_permission: i64,
    /// The time the channel was last live
    #[sqlx(rename = "channel_last_live_at")]
    pub last_live_at: Option<DateTime<Utc>>,
}

impl Channel {
    pub fn get_stream_key(&self) -> Option<String> {
        self.stream_key
            .as_ref()
            .map(|s| format!("live_{}_{}", self.id.0, s))
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, SimpleObject)]
pub struct ChannelLink {
    #[serde(rename = "n")]
    pub name: String,
    #[serde(rename = "u")]
    pub url: String,
}
