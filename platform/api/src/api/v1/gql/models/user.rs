use async_graphql::{ComplexObject, Context, SimpleObject};
use ulid::Ulid;

use super::channel::Channel;
use super::color::DisplayColor;
use super::date::DateRFC3339;
use super::image_upload::ImageUpload;
use super::ulid::GqlUlid;
use crate::api::v1::gql::error::ext::ResultExt;
use crate::api::v1::gql::error::Result;
use crate::api::v1::gql::ext::ContextExt;
use crate::api::v1::gql::guards::auth_guard;
use crate::database;
use crate::global::ApiGlobal;

#[derive(SimpleObject, Clone)]
#[graphql(complex)]
pub struct User<G: ApiGlobal> {
	pub id: GqlUlid,
	pub display_name: String,
	pub display_color: DisplayColor,
	pub username: String,
	pub channel: Channel<G>,

	// Private fields
	#[graphql(skip)]
	pub email_: String,
	#[graphql(skip)]
	pub email_verified_: bool,
	#[graphql(skip)]
	pub last_login_at_: DateRFC3339,
	#[graphql(skip)]
	pub totp_enabled_: bool,
	#[graphql(skip)]
	pub profile_picture_: Option<Ulid>,
}

#[ComplexObject]
impl<G: ApiGlobal> User<G> {
	async fn email(&self, ctx: &Context<'_>) -> Result<&str> {
		auth_guard::<_, G>(ctx, "email", self.email_.as_str(), self.id.into()).await
	}

	async fn email_verified(&self, ctx: &Context<'_>) -> Result<bool> {
		auth_guard::<_, G>(ctx, "emailVerified", self.email_verified_, self.id.into()).await
	}

	async fn last_login_at(&self, ctx: &Context<'_>) -> Result<&DateRFC3339> {
		auth_guard::<_, G>(ctx, "lastLoginAt", &self.last_login_at_, self.id.into()).await
	}

	async fn totp_enabled(&self, ctx: &Context<'_>) -> Result<bool> {
		auth_guard::<_, G>(ctx, "totpEnabled", self.totp_enabled_, self.id.into()).await
	}

	async fn profile_picture(&self, ctx: &Context<'_>) -> Result<Option<ImageUpload>> {
		let Some(profile_picture_id) = self.profile_picture_ else {
			return Ok(None);
		};

		let global = ctx.get_global::<G>();

		Ok(global
			.uploaded_file_by_id_loader()
			.load(profile_picture_id)
			.await
			.map_err_ignored_gql("failed to fetch profile picture")?
			.map(ImageUpload::from_uploaded_file)
			.transpose()?
			.flatten())
	}
}

impl<G: ApiGlobal> From<database::User> for User<G> {
	fn from(value: database::User) -> Self {
		Self {
			id: value.id.0.into(),
			username: value.username,
			display_name: value.display_name,
			display_color: value.display_color.into(),
			channel: value.channel.into(),
			email_: value.email,
			email_verified_: value.email_verified,
			last_login_at_: value.last_login_at.into(),
			totp_enabled_: value.totp_enabled,
			profile_picture_: value.profile_picture_id.map(|u| u.0),
		}
	}
}
