use async_graphql::{ComplexObject, Context, SimpleObject};
use chrono::Utc;

use super::category::Category;
use super::date::DateRFC3339;
use super::ulid::GqlUlid;
use crate::api::v1::gql::error::ext::*;
use crate::api::v1::gql::error::Result;
use crate::api::v1::gql::ext::ContextExt;
use crate::api::v1::gql::guards::auth_guard;
use crate::database;
use crate::global::ApiGlobal;

#[derive(SimpleObject, Clone)]
#[graphql(complex)]
pub struct Channel<G: ApiGlobal> {
	pub id: GqlUlid,
	pub room_id: GqlUlid,
	pub title: Option<String>,
	pub live_viewer_count_updated_at: Option<DateRFC3339>,
	pub description: Option<String>,
	pub links: Vec<database::ChannelLink>,
	pub custom_thumbnail_id: Option<GqlUlid>,
	pub offline_banner_id: Option<GqlUlid>,
	pub category_id: Option<GqlUlid>,
	pub last_live_at: Option<DateRFC3339>,

	// Live viewer count has a custom resolver
	#[graphql(skip)]
	live_viewer_count_: Option<i32>,

	// Private fields
	#[graphql(skip)]
	stream_key_: Option<String>,

	#[graphql(skip)]
	_phantom: std::marker::PhantomData<G>,
}

#[ComplexObject]
impl<G: ApiGlobal> Channel<G> {
	async fn category(&self, ctx: &Context<'_>) -> Result<Option<Category>> {
		let global = ctx.get_global::<G>();

		let Some(category_id) = self.category_id else {
			return Ok(None);
		};

		let category = global
			.category_by_id_loader()
			.load(category_id.into())
			.await
			.map_err_ignored_gql("failed to fetch category")?;

		Ok(category.map(Into::into))
	}

	async fn stream_key(&self, ctx: &Context<'_>) -> Result<Option<&str>> {
		auth_guard::<_, G>(ctx, "streamKey", self.stream_key_.as_deref(), self.id.into()).await
	}

	async fn followers_count(&self, ctx: &Context<'_>) -> Result<i64> {
		let global = ctx.get_global::<G>();

		let (followers,) = sqlx::query_as(
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
		.bind(self.id.to_uuid())
		.fetch_one(global.db().as_ref())
		.await
		.map_err_gql("failed to fetch followers")?;

		Ok(followers)
	}

	async fn live(&self, ctx: &Context<'_>) -> Result<bool> {
		let global = ctx.get_global::<G>();

		let res = global
			.video_room_client()
			.clone()
			.get(pb::scuffle::video::v1::RoomGetRequest {
				ids: vec![self.room_id.0.into()],
				..Default::default()
			})
			.await
			.map_err_gql("failed to fetch room")?;
		let room = res.into_inner().rooms.into_iter().next().map_err_gql("failed to fetch room")?;

		Ok(room.status == pb::scuffle::video::v1::types::RoomStatus::Ready as i32)
	}

	async fn live_viewer_count(&self, ctx: &Context<'_>) -> Result<i32> {
		let global = ctx.get_global::<G>();

		if let Some(count) = self.live_viewer_count_ {
			let expired = self
				.live_viewer_count_updated_at
				.as_ref()
				.map(|DateRFC3339(t)| Utc::now().signed_duration_since(t).num_seconds() > 30)
				.unwrap_or(true);
			if !expired {
				return Ok(count);
			}
		}

		let res = global
			.video_playback_session_client()
			.clone()
			.count(pb::scuffle::video::v1::PlaybackSessionCountRequest {
				filter: Some(pb::scuffle::video::v1::playback_session_count_request::Filter::Target(
					pb::scuffle::video::v1::types::PlaybackSessionTarget {
						target: Some(pb::scuffle::video::v1::types::playback_session_target::Target::RoomId(
							self.room_id.0.into(),
						)),
					},
				)),
			})
			.await
			.map_err_gql("failed to fetch playback session count")?;

		let live_viewer_count = res.into_inner().deduplicated_count as i32; //should be safe to cast

		sqlx::query("UPDATE users SET channel_live_viewer_count = $1, channel_live_viewer_count_updated_at = NOW() WHERE id = $2")
			.bind(live_viewer_count)
			.bind(self.id.to_uuid())
			.execute(global.db().as_ref())
			.await
			.map_err_gql("failed to update live viewer count")?;

		Ok(live_viewer_count)
	}
}

impl<G: ApiGlobal> From<database::Channel> for Channel<G> {
	fn from(value: database::Channel) -> Self {
		let stream_key_ = value.get_stream_key();
		Self {
			id: value.id.0.into(),
			room_id: value.room_id.0.into(),
			title: value.title,
			live_viewer_count_: value.live_viewer_count,
			live_viewer_count_updated_at: value.live_viewer_count_updated_at.map(DateRFC3339),
			description: value.description,
			links: value.links.0,
			custom_thumbnail_id: value.custom_thumbnail_id.map(|v| Into::into(v.0)),
			offline_banner_id: value.offline_banner_id.map(|v| Into::into(v.0)),
			category_id: value.category_id.map(|v| Into::into(v.0)),
			last_live_at: value.last_live_at.map(DateRFC3339),
			stream_key_,
			_phantom: std::marker::PhantomData,
		}
	}
}
