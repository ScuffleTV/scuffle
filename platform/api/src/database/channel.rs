use async_graphql::SimpleObject;
use chrono::{DateTime, Utc};
use utils::database::json;
use ulid::Ulid;

#[derive(Debug, Clone, Default, postgres_from_row::FromRow)]
pub struct Channel {
	/// Ulid of the channel
	pub id: Ulid,
	/// Video room id
	#[from_row(rename = "channel_room_id")]
	pub room_id: Ulid,
	/// Active connection id
	#[from_row(rename = "channel_active_connection_id")]
	pub active_connection_id: Option<Ulid>,
	/// The current stream's title
	#[from_row(rename = "channel_title")]
	pub title: Option<String>,
	/// The current stream's live viewer count
	#[from_row(rename = "channel_live_viewer_count")]
	pub live_viewer_count: Option<i32>,
	/// The time the current stream's live viewer count was last updated
	#[from_row(rename = "channel_live_viewer_count_updated_at")]
	pub live_viewer_count_updated_at: Option<DateTime<Utc>>,
	/// The current stream's description
	#[from_row(rename = "channel_description")]
	pub description: Option<String>,
	/// The social links
	#[from_row(rename = "channel_links", from_fn = "json")]
	pub links: Vec<ChannelLink>,
	/// The current stream's thumbnail
	#[from_row(rename = "channel_custom_thumbnail_id")]
	pub custom_thumbnail_id: Option<Ulid>,
	/// The offline banner of the channel
	#[from_row(rename = "channel_offline_banner_id")]
	pub offline_banner_id: Option<Ulid>,
	/// The current stream's category
	#[from_row(rename = "channel_category_id")]
	pub category_id: Option<Ulid>,
	/// Channel stream key
	#[from_row(rename = "channel_stream_key")]
	pub stream_key: Option<String>,
	/// Channel roles order
	#[from_row(rename = "channel_role_order")]
	pub role_order: Vec<Ulid>,
	/// Channel default permissions
	#[from_row(rename = "channel_default_permissions")]
	pub default_permissions: i64,
	/// Channel permissions for followers
	#[from_row(rename = "channel_following_permission")]
	pub following_permission: i64,
	/// The time the channel was last live
	#[from_row(rename = "channel_last_live_at")]
	pub last_live_at: Option<DateTime<Utc>>,
}

impl Channel {
	pub fn get_stream_key(&self) -> Option<String> {
		self.stream_key.as_ref().map(|s| format!("live_{}_{}", self.id.0, s))
	}
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, SimpleObject)]
pub struct ChannelLink {
	#[serde(rename = "n")]
	pub name: String,
	#[serde(rename = "u")]
	pub url: String,
}
