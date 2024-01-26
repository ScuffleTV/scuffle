use async_graphql::{ComplexObject, Context, SimpleObject};
use chrono::Utc;
use jwt_next::SignWithKey;
use ulid::Ulid;

use super::category::Category;
use super::date::DateRFC3339;
use super::image_upload::ImageUpload;
use super::ulid::GqlUlid;
use crate::api::v1::gql::error::ext::*;
use crate::api::v1::gql::error::Result;
use crate::api::v1::gql::ext::ContextExt;
use crate::api::v1::gql::guards::auth_guard;
use crate::config::{VideoApiConfig, VideoApiPlaybackKeypairConfig};
use crate::database;
use crate::global::ApiGlobal;
use crate::video_api::request_deduplicated_viewer_count;

#[derive(SimpleObject, Clone)]
#[graphql(complex)]
pub struct Channel<G: ApiGlobal> {
	pub id: GqlUlid,
	pub title: Option<String>,
	pub description: Option<String>,
	pub links: Vec<database::ChannelLink>,
	pub custom_thumbnail_id: Option<GqlUlid>,
	pub pending_offline_banner_id: Option<GqlUlid>,
	pub category_id: Option<GqlUlid>,
	pub live: Option<ChannelLive<G>>,
	pub last_live_at: Option<DateRFC3339>,

	// Custom resolver
	#[graphql(skip)]
	pub offline_banner_id_: Option<Ulid>,

	// Private fields
	#[graphql(skip)]
	stream_key_: Option<String>,
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

	async fn offline_banner(&self, ctx: &Context<'_>) -> Result<Option<ImageUpload<G>>> {
		let Some(offline_banner_id) = self.offline_banner_id_ else {
			return Ok(None);
		};

		let global = ctx.get_global::<G>();

		Ok(global
			.uploaded_file_by_id_loader()
			.load(offline_banner_id)
			.await
			.map_err_ignored_gql("failed to fetch offline banner")?
			.map(ImageUpload::from_uploaded_file)
			.transpose()?
			.flatten())
	}

	async fn stream_key(&self, ctx: &Context<'_>) -> Result<Option<&str>> {
		auth_guard::<_, G>(ctx, "streamKey", self.stream_key_.as_deref(), self.id.into()).await
	}

	async fn followers_count(&self, ctx: &Context<'_>) -> Result<i64> {
		let global = ctx.get_global::<G>();

		let followers = common::database::query(
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
		.bind(self.id.to_ulid())
		.build_query_single_scalar()
		.fetch_one(global.db())
		.await
		.map_err_gql("failed to fetch followers")?;

		Ok(followers)
	}
}

#[derive(Clone, SimpleObject)]
#[graphql(complex)]
pub struct ChannelLive<G: ApiGlobal> {
	pub room_id: GqlUlid,

	// Live viewer count has a custom resolver
	#[graphql(skip)]
	live_viewer_count_: Option<i32>,
	#[graphql(skip)]
	live_viewer_count_updated_at_: Option<DateRFC3339>,
	// Needed for the live_viewer_count resolver
	#[graphql(skip)]
	channel_id: ulid::Ulid,

	#[graphql(skip)]
	_phantom: std::marker::PhantomData<G>,
}

#[derive(serde::Serialize)]
struct PlayerTokenClaim {
	organization_id: String,
	room_id: String,
	iat: i64,
	user_id: String,
}

#[ComplexObject]
impl<G: ApiGlobal> ChannelLive<G> {
	async fn live_viewer_count(&self, ctx: &Context<'_>) -> Result<i32> {
		let global = ctx.get_global::<G>();

		if let Some(count) = self.live_viewer_count_ {
			let expired = self
				.live_viewer_count_updated_at_
				.as_ref()
				.map(|DateRFC3339(t)| Utc::now().signed_duration_since(t).num_seconds() > 30)
				.unwrap_or(true);
			if !expired {
				return Ok(count);
			}
		}

		let live_viewer_count =
			request_deduplicated_viewer_count(&mut global.video_playback_session_client().clone(), self.room_id.0)
				.await
				.map_err_gql("failed to fetch playback session count")?;

		common::database::query(
			"UPDATE users SET channel_live_viewer_count = $1, channel_live_viewer_count_updated_at = NOW() WHERE id = $2",
		)
		.bind(live_viewer_count)
		.bind(self.channel_id)
		.build()
		.execute(global.db())
		.await
		.map_err_gql("failed to update live viewer count")?;

		Ok(live_viewer_count)
	}

	async fn player_token(&self, ctx: &Context<'_>) -> Result<Option<String>> {
		let global = ctx.get_global::<G>();

		let request_context = ctx.get_req_context();
		let Some(auth) = request_context.auth(global).await? else {
			// If the request is not authenticated, return None
			return Ok(None);
		};

		let video_api_config: &VideoApiConfig = global.provide_config();
		let (Some(VideoApiPlaybackKeypairConfig { id: public_key_id, .. }), Some(private_key)) =
			(video_api_config.playback_keypair.as_ref(), global.playback_private_key())
		else {
			// If the video api playback keypair is not configured, return None
			return Ok(None);
		};

		let header = jwt_next::Header {
			algorithm: jwt_next::AlgorithmType::Es384,
			key_id: Some(public_key_id.to_string()),
			type_: Some(jwt_next::header::HeaderType::JsonWebToken),
			..Default::default()
		};

		let token = jwt_next::Token::new(
			header,
			PlayerTokenClaim {
				organization_id: video_api_config.organization_id.to_string(),
				room_id: self.room_id.to_string(),
				iat: Utc::now().timestamp(),
				user_id: auth.session.user_id.to_string(),
			},
		)
		.sign_with_key(private_key)
		.map_err_ignored_gql("failed to sign token")?;

		Ok(Some(token.as_str().to_string()))
	}

	async fn edge_endpoint(&self, ctx: &Context<'_>) -> Result<String> {
		let global = ctx.get_global::<G>();

		let video_api_config: &VideoApiConfig = global.provide_config();

		Ok(video_api_config.edge_endpoint.clone())
	}

	async fn organization_id(&self, ctx: &Context<'_>) -> Result<GqlUlid> {
		let global = ctx.get_global::<G>();

		let video_api_config: &VideoApiConfig = global.provide_config();

		Ok(video_api_config.organization_id.into())
	}
}

impl<G: ApiGlobal> From<database::Channel> for Channel<G> {
	fn from(value: database::Channel) -> Self {
		let stream_key_ = value.get_stream_key();
		Self {
			id: value.id.into(),
			title: value.title,
			description: value.description,
			links: value.links,
			custom_thumbnail_id: value.custom_thumbnail_id.map(Into::into),
			pending_offline_banner_id: value.pending_offline_banner_id.map(Into::into),
			category_id: value.category_id.map(Into::into),
			live: value.active_connection_id.map(|_| ChannelLive {
				room_id: value.room_id.into(),
				live_viewer_count_: value.live_viewer_count,
				live_viewer_count_updated_at_: value.live_viewer_count_updated_at.map(DateRFC3339),
				channel_id: value.id,
				_phantom: std::marker::PhantomData,
			}),
			offline_banner_id_: value.offline_banner_id,
			last_live_at: value.last_live_at.map(DateRFC3339),
			stream_key_,
		}
	}
}
