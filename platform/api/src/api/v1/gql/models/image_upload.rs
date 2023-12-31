use async_graphql::{Enum, SimpleObject};
use pb::scuffle::platform::internal::types::uploaded_file_metadata::{self, Image as PbImage};
use ulid::Ulid;

use super::ulid::GqlUlid;
use crate::api::v1::gql::error::{GqlError, Result};
use crate::database::UploadedFile;

#[derive(SimpleObject, Clone)]
pub struct ImageUpload {
	pub id: GqlUlid,
	pub variants: Vec<ImageUploadVariant>,
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

impl ImageUpload {
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
