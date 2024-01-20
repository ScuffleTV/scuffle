use std::sync::Arc;

use common::database::deadpool_postgres::Transaction;
use common::http::ext::ResultExt;
use common::http::RouteError;
use hyper::StatusCode;
use pb::scuffle::platform::internal::image_processor;
use pb::scuffle::platform::internal::types::ImageFormat;
use ulid::Ulid;

use super::{AcceptedFormats, ImageUploadRequest};
use crate::api::auth::AuthData;
use crate::api::error::ApiError;
use crate::config::{ApiConfig, ImageUploaderConfig};
use crate::database::{FileType, RolePermission};
use crate::global::ApiGlobal;

#[derive(Default, serde::Deserialize)]
#[serde(default)]
pub struct ProfilePicture {
	set_active: bool,
}

impl ImageUploadRequest for ProfilePicture {
	fn create_task<G: ApiGlobal>(
		global: &Arc<G>,
		auth: &AuthData,
		format: AcceptedFormats,
		file_id: Ulid,
		owner_id: Ulid,
	) -> image_processor::Task {
		let config = global.config::<ImageUploaderConfig>();

		image_processor::Task {
			input_path: format!(
				"{}/profile_pictures/{}/source.{}",
				auth.session.user_id,
				file_id,
				format.ext()
			),
			base_height: 128, // 128, 256, 384, 512
			base_width: 128,  // 128, 256, 384, 512
			formats: vec![
				ImageFormat::PngStatic as i32,
				ImageFormat::AvifStatic as i32,
				ImageFormat::WebpStatic as i32,
				ImageFormat::Gif as i32,
				ImageFormat::Webp as i32,
				ImageFormat::Avif as i32,
			],
			callback_subject: format!("{}.{}", config.callback_subject, config.profile_picture_suffix),
			limits: Some(image_processor::task::Limits {
				max_input_duration_ms: 10 * 1000, // 10 seconds
				max_input_frame_count: 300,
				max_input_height: 1000,
				max_input_width: 1000,
				max_processing_time_ms: 60 * 1000, // 60 seconds
			}),
			resize_algorithm: image_processor::task::ResizeAlgorithm::Lanczos3 as i32,
			upscale: true, // For profile pictures we want to have a consistent size
			scales: vec![1, 2, 3, 4],
			resize_method: image_processor::task::ResizeMethod::PadCenter as i32,
			output_prefix: format!("{owner_id}/{file_id}"),
		}
	}

	fn task_priority<G: ApiGlobal>(global: &std::sync::Arc<G>) -> i64 {
		global.config::<ImageUploaderConfig>().profile_picture_task_priority
	}

	fn get_max_size<G: ApiGlobal>(global: &Arc<G>) -> usize {
		global.config::<ApiConfig>().max_profile_picture_size
	}

	fn validate_permissions(auth: &AuthData) -> bool {
		auth.user_permissions.has_permission(RolePermission::UploadProfilePicture)
	}

	fn file_type<G: ApiGlobal>(_global: &std::sync::Arc<G>) -> FileType {
		FileType::ProfilePicture
	}

	async fn process(&self, auth: &AuthData, tx: &Transaction<'_>, file_id: Ulid) -> Result<(), RouteError<ApiError>> {
		if self.set_active {
			common::database::query("UPDATE users SET pending_profile_picture_id = $1 WHERE id = $2")
				.bind(file_id)
				.bind(auth.session.user_id)
				.build()
				.execute(&tx)
				.await
				.map_err_route((StatusCode::INTERNAL_SERVER_ERROR, "failed to update user"))?;
		}
		Ok(())
	}
}
