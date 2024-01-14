use async_graphql::{Enum, SimpleObject, ComplexObject, Context};
use pb::scuffle::platform::internal::types::uploaded_file_metadata::{self, Image as PbImage};
use ulid::Ulid;

use super::ulid::GqlUlid;
use crate::api::v1::gql::error::{GqlError, Result};
use crate::config::ImageUploaderConfig;
use crate::database::UploadedFile;
use crate::global::ApiGlobal;
use crate::api::v1::gql::ext::ContextExt;

#[derive(SimpleObject, Clone)]
#[graphql(complex)]
pub struct ImageUpload<G: ApiGlobal> {
	pub id: GqlUlid,
	pub variants: Vec<ImageUploadVariant>,

	#[graphql(skip)]
	_phantom: std::marker::PhantomData<G>,
}

#[derive(SimpleObject, Clone)]
pub struct ImageUploadVariant {
	pub width: u32,
	pub height: u32,
	pub scale: u32,
	pub url: String,
	pub format: ImageUploadFormat,
	pub byte_size: u32,
}

#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ImageUploadFormat {
	PngStatic,
	WebpStatic,
	AvifStatic,
	Gif,
	Webp,
	Avif,
}

#[ComplexObject]
impl<G: ApiGlobal> ImageUpload<G> {
	async fn endpoint<'a>(&self, ctx: &'a Context<'_>) -> &'a str {
		let global = ctx.get_global::<G>();

		let config: &ImageUploaderConfig = global.provide_config();
		&config.public_endpoint
	}
}

impl<G: ApiGlobal> ImageUpload<G> {
	pub fn from_uploaded_file(uploaded_file: UploadedFile) -> Result<Option<Self>> {
		if uploaded_file.pending {
			return Ok(None);
		}

		if let Some(uploaded_file_metadata::Metadata::Image(image)) = uploaded_file.metadata.0.metadata {
			Ok(Some(Self::new(uploaded_file.id.0, image)))
		} else {
			Err(GqlError::InternalServerError("uploaded file is not an image").into())
		}
	}

	pub fn new(id: Ulid, image: PbImage) -> Self {
		Self {
			id: GqlUlid(id),
			variants: image.versions.into_iter().map(Into::into).collect(),
			_phantom: std::marker::PhantomData,
		}
	}
}

impl From<pb::scuffle::platform::internal::types::ProcessedImageVariant> for ImageUploadVariant {
	fn from(value: pb::scuffle::platform::internal::types::ProcessedImageVariant) -> Self {
		Self {
			width: value.width,
			height: value.height,
			scale: value.scale,
			format: value.format().into(),
			byte_size: value.byte_size,
			url: value.path,
		}
	}
}

impl From<pb::scuffle::platform::internal::types::ImageFormat> for ImageUploadFormat {
	fn from(value: pb::scuffle::platform::internal::types::ImageFormat) -> Self {
		match value {
			pb::scuffle::platform::internal::types::ImageFormat::PngStatic => Self::PngStatic,
			pb::scuffle::platform::internal::types::ImageFormat::WebpStatic => Self::WebpStatic,
			pb::scuffle::platform::internal::types::ImageFormat::AvifStatic => Self::AvifStatic,
			pb::scuffle::platform::internal::types::ImageFormat::Gif => Self::Gif,
			pb::scuffle::platform::internal::types::ImageFormat::Webp => Self::Webp,
			pb::scuffle::platform::internal::types::ImageFormat::Avif => Self::Avif,
		}
	}
}
