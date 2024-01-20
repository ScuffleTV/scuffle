use std::sync::Arc;

use aws_sdk_s3::types::ObjectCannedAcl;
use bytes::Bytes;
use common::database::deadpool_postgres::Transaction;
use common::http::ext::ResultExt;
use common::http::RouteError;
use common::make_response;
use common::s3::PutObjectOptions;
use hyper::{Response, StatusCode};
use pb::scuffle::platform::internal::image_processor;
use pb::scuffle::platform::internal::types::{uploaded_file_metadata, UploadedFileMetadata};
use serde_json::json;
use ulid::Ulid;

use super::UploadType;
use crate::api::auth::AuthData;
use crate::api::error::ApiError;
use crate::api::Body;
use crate::database::{FileType, UploadedFileStatus};
use crate::global::ApiGlobal;

pub(crate) mod offline_banner;
pub(crate) mod profile_picture;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum AcceptedFormats {
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

pub(super) trait ImageUploadRequest {
	fn create_task<G: ApiGlobal>(
		global: &Arc<G>,
		auth: &AuthData,
		format: AcceptedFormats,
		file_id: Ulid,
		owner_id: Ulid,
	) -> image_processor::Task;

	fn task_priority<G: ApiGlobal>(global: &Arc<G>) -> i64;

	fn get_max_size<G: ApiGlobal>(global: &Arc<G>) -> usize;

	fn validate_permissions(auth: &AuthData) -> bool;

	fn file_type<G: ApiGlobal>(global: &Arc<G>) -> FileType;

	async fn process(&self, auth: &AuthData, tx: &Transaction, file_id: Ulid) -> Result<(), RouteError<ApiError>>;
}

impl<T: ImageUploadRequest + serde::de::DeserializeOwned + Default> UploadType for T {
	fn validate_format<G: ApiGlobal>(_global: &Arc<G>, _auth: &AuthData, content_type: &str) -> bool {
		AcceptedFormats::from_content_type(content_type).is_some()
	}

	fn validate_permissions(&self, auth: &AuthData) -> bool {
		T::validate_permissions(auth)
	}

	fn get_max_size<G: ApiGlobal>(global: &Arc<G>) -> usize {
		T::get_max_size(global)
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

		let task = T::create_task(global, &auth, image_format, file_id, auth.session.user_id);

		let input_path = task.input_path.clone();

		let mut client = global
			.db()
			.get()
			.await
			.map_err_route((StatusCode::INTERNAL_SERVER_ERROR, "failed to get database connection"))?;
		let tx = client
			.transaction()
			.await
			.map_err_route((StatusCode::INTERNAL_SERVER_ERROR, "failed to start transaction"))?;

		common::database::query("INSERT INTO image_jobs (id, priority, task) VALUES ($1, $2, $3)")
			.bind(file_id)
			.bind(T::task_priority(global))
			.bind(common::database::Protobuf(task))
			.build()
			.execute(&tx)
			.await
			.map_err_route((StatusCode::INTERNAL_SERVER_ERROR, "failed to insert image job"))?;

		common::database::query("INSERT INTO uploaded_files(id, owner_id, uploader_id, name, type, metadata, total_size, path, status) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)")
            .bind(file_id) // id
            .bind(auth.session.user_id) // owner_id
            .bind(auth.session.user_id) // uploader_id
            .bind(name.unwrap_or_else(|| format!("untitled.{}", image_format.ext()))) // name
            .bind(T::file_type(global)) // type
            .bind(common::database::Protobuf(UploadedFileMetadata {
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

		T::process(&self, &auth, &tx, file_id).await?;

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
