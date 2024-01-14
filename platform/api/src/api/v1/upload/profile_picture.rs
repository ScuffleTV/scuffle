use std::sync::Arc;

use aws_sdk_s3::types::ObjectCannedAcl;
use binary_helper::s3::PutObjectOptions;
use bytes::Bytes;
use hyper::{Response, StatusCode};
use pb::scuffle::platform::internal::image_processor;
use pb::scuffle::platform::internal::types::{uploaded_file_metadata, ImageFormat, UploadedFileMetadata};
use serde_json::json;
use ulid::Ulid;
use utils::http::ext::ResultExt;
use utils::http::RouteError;
use utils::make_response;

use super::UploadType;
use crate::api::auth::AuthData;
use crate::api::error::ApiError;
use crate::api::Body;
use crate::config::{ApiConfig, ImageUploaderConfig};
use crate::database::{FileType, RolePermission, UploadedFileStatus};
use crate::global::ApiGlobal;

fn create_task(file_id: Ulid, input_path: &str, config: &ImageUploaderConfig, owner_id: Ulid) -> image_processor::Task {
	image_processor::Task {
		input_path: input_path.to_string(),
		aspect_ratio: Some(image_processor::task::Ratio {
			numerator: 1,
			denominator: 1,
		}),
		clamp_aspect_ratio: true,
		formats: vec![
			ImageFormat::PngStatic as i32,
			ImageFormat::AvifStatic as i32,
			ImageFormat::WebpStatic as i32,
			ImageFormat::Gif as i32,
			ImageFormat::Webp as i32,
			ImageFormat::Avif as i32,
		],
		callback_subject: config.callback_subject.clone(),
		limits: Some(image_processor::task::Limits {
			max_input_duration_ms: 10 * 1000, // 10 seconds
			max_input_frame_count: 300,
			max_input_height: 1000,
			max_input_width: 1000,
			max_processing_time_ms: 60 * 1000, // 60 seconds
		}),
		resize_algorithm: image_processor::task::ResizeAlgorithm::Lanczos3 as i32,
		upscale: image_processor::task::Upscale::NoPreserveSource as i32,
		input_image_scaling: true,
		scales: vec![
			64,
			128,
			256,
			384,
		],
		resize_method: image_processor::task::ResizeMethod::PadCenter as i32,
		output_prefix: format!("{owner_id}/{file_id}"),
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum AcceptedFormats {
	Webp,
	Avif,
	Avifs,
	Gif,
	Png,
	Apng,
	Jls,
	Jpeg,
	Jxl,
	Bmp,
	Heic,
	Heics,
	Heif,
	Heifs,
	Mp4,
	Mp4v,
	Flv,
	Mkv,
	Avi,
	Mov,
	Webm,
	M2ts,
}

impl AcceptedFormats {
	pub fn from_content_type(content_type: &str) -> Option<Self> {
		match content_type {
			"image/webp" => Some(Self::Webp),
			"image/avif" => Some(Self::Avif),
			"image/avif-sequence" => Some(Self::Avifs),
			"image/gif" => Some(Self::Gif),
			"image/png" => Some(Self::Png),
			"image/apng" => Some(Self::Apng),
			"image/jls" => Some(Self::Jls),
			"image/jpeg" => Some(Self::Jpeg),
			"image/jxl" => Some(Self::Jxl),
			"image/bmp" => Some(Self::Bmp),
			"image/heic" => Some(Self::Heic),
			"image/heic-sequence" => Some(Self::Heics),
			"image/heif" => Some(Self::Heif),
			"image/heif-sequence" => Some(Self::Heifs),
			"application/mp4" => Some(Self::Mp4),
			"video/mp4" => Some(Self::Mp4v),
			"video/x-flv" => Some(Self::Flv),
			"video/x-matroska" => Some(Self::Mkv),
			"video/avi" => Some(Self::Avi),
			"video/quicktime" => Some(Self::Mov),
			"video/webm" => Some(Self::Webm),
			"video/mp2t" => Some(Self::M2ts),
			_ => None,
		}
	}

	pub const fn ext(self) -> &'static str {
		match self {
			Self::Webp => "webp",
			Self::Avif => "avif",
			Self::Avifs => "avifs",
			Self::Gif => "gif",
			Self::Png => "png",
			Self::Apng => "apng",
			Self::Jls => "jls",
			Self::Jpeg => "jpg",
			Self::Jxl => "jxl",
			Self::Bmp => "bmp",
			Self::Heic => "heic",
			Self::Heics => "heics",
			Self::Heif => "heif",
			Self::Heifs => "heifs",
			Self::Mp4 => "mp4",
			Self::Mp4v => "mp4v",
			Self::Flv => "flv",
			Self::Mkv => "mkv",
			Self::Avi => "avi",
			Self::Mov => "mov",
			Self::Webm => "webm",
			Self::M2ts => "m2ts",
		}
	}
}

#[derive(Default, serde::Deserialize)]
#[serde(default)]
pub(super) struct ProfilePicture {
	set_active: bool,
}

impl UploadType for ProfilePicture {
	fn validate_format<G: ApiGlobal>(_: &Arc<G>, _: &AuthData, content_type: &str) -> bool {
		AcceptedFormats::from_content_type(content_type).is_some()
	}

	fn validate_permissions(&self, auth: &AuthData) -> bool {
		auth.user_permissions.has_permission(RolePermission::UploadProfilePicture)
	}

	fn get_max_size<G: ApiGlobal>(global: &Arc<G>) -> usize {
		global.config::<ApiConfig>().max_profile_picture_size
	}

	async fn handle<G: ApiGlobal>(
		self,
		global: &Arc<G>,
		auth: AuthData,
		name: Option<String>,
		file: Bytes,
		content_type: &str,
	) -> Result<Response<Body>, RouteError<ApiError>> {
		let image_format = AcceptedFormats::from_content_type(content_type)
			.ok_or((StatusCode::BAD_REQUEST, "invalid content-type header"))?;

		let file_id = Ulid::new();

		let config = global.config::<ImageUploaderConfig>();

		let input_path = format!(
			"{}/profile_pictures/{}/source.{}",
			auth.session.user_id,
			file_id,
			image_format.ext()
		);

		let mut client = global
			.db()
			.get()
			.await
			.map_err_route((StatusCode::INTERNAL_SERVER_ERROR, "failed to get database connection"))?;
		let tx = client
			.transaction()
			.await
			.map_err_route((StatusCode::INTERNAL_SERVER_ERROR, "failed to start transaction"))?;

		utils::database::query("INSERT INTO image_jobs (id, priority, task) VALUES ($1, $2, $3)")
			.bind(file_id)
			.bind(config.profile_picture_task_priority)
			.bind(utils::database::Protobuf(create_task(
				file_id,
				&input_path,
				config,
				auth.session.user_id,
			)))
			.build()
			.execute(&tx)
			.await
			.map_err_route((StatusCode::INTERNAL_SERVER_ERROR, "failed to insert image job"))?;

		utils::database::query("INSERT INTO uploaded_files(id, owner_id, uploader_id, name, type, metadata, total_size, path, status) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)")
            .bind(file_id) // id
            .bind(auth.session.user_id) // owner_id
            .bind(auth.session.user_id) // uploader_id
            .bind(name.unwrap_or_else(|| format!("untitled.{}", image_format.ext()))) // name
            .bind(FileType::ProfilePicture) // type
            .bind(utils::database::Protobuf(UploadedFileMetadata {
				metadata: Some(uploaded_file_metadata::Metadata::Image(uploaded_file_metadata::Image {
					versions: Vec::new(),
				})),
			})) // metadata
            .bind(file.len() as i64) // total_size
            .bind(&input_path) // path
			.bind(UploadedFileStatus::Queued) // status
			.build()
            .execute(&tx)
            .await
            .map_err_route((StatusCode::INTERNAL_SERVER_ERROR, "failed to insert uploaded file"))?;

		if self.set_active {
			utils::database::query("UPDATE users SET pending_profile_picture_id = $1 WHERE id = $2")
				.bind(file_id)
				.bind(auth.session.user_id)
				.build()
				.execute(&tx)
				.await
				.map_err_route((StatusCode::INTERNAL_SERVER_ERROR, "failed to update user"))?;
		}

		global
			.image_uploader_s3()
			.put_object(
				&input_path,
				file,
				Some(PutObjectOptions {
					acl: Some(ObjectCannedAcl::Private),
					content_type: Some(content_type.to_owned()),
				}),
			)
			.await
			.map_err(|err| {
				tracing::error!(error = %err, "failed to upload image to s3");
				(StatusCode::INTERNAL_SERVER_ERROR, "failed to upload image to s3")
			})?;

		tx.commit()
			.await
			.map_err(|err| {
				tracing::warn!(path = %input_path, "possible leaked s3 upload");
				err
			})
			.map_err_route((StatusCode::INTERNAL_SERVER_ERROR, "failed to commit transaction"))?;

		Ok(make_response!(
			StatusCode::OK,
			json!({
				"success": true,
				"file_id": file_id.to_string(),
			})
		))
	}
}
