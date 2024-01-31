use std::sync::Arc;

use binary_helper::global::RequestGlobalExt;
use bytes::Bytes;
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};
use multer::{Constraints, SizeLimit};
use utils::http::ext::{OptionExt, ResultExt};
use utils::http::router::builder::RouterBuilder;
use utils::http::router::compat::BodyExt;
use utils::http::router::ext::RequestExt;
use utils::http::router::Router;
use utils::http::RouteError;

use self::profile_picture::ProfilePicture;
use crate::api::auth::AuthData;
use crate::api::error::ApiError;
use crate::api::request_context::RequestContext;
use crate::api::Body;
use crate::global::ApiGlobal;
use crate::turnstile::validate_turnstile_token;

pub(crate) mod profile_picture;

trait UploadType: serde::de::DeserializeOwned + Default {
	fn validate_format<G: ApiGlobal>(global: &Arc<G>, auth: &AuthData, content_type: &str) -> bool;

	fn get_max_size<G: ApiGlobal>(global: &Arc<G>) -> usize;

	fn validate_permissions(&self, auth: &AuthData) -> bool;

	async fn handle<G: ApiGlobal>(
		self,
		global: &Arc<G>,
		auth: AuthData,
		file_name: Option<String>,
		file: Bytes,
		content_type: &str,
	) -> Result<Response<Body>, RouteError<ApiError>>;
}

pub fn routes<G: ApiGlobal>(_: &Arc<G>) -> RouterBuilder<Incoming, Body, RouteError<ApiError>> {
	Router::builder().post("/profile-picture", handler::<G, ProfilePicture>)
}

async fn handler<G: ApiGlobal, U: UploadType>(req: Request<Incoming>) -> Result<Response<Body>, RouteError<ApiError>> {
	let global = req.get_global::<G, _>()?;

	let request_context = req.data::<RequestContext>().expect("missing request context");

	let auth = request_context
		.auth(&global)
		.await?
		.map_err_route((StatusCode::UNAUTHORIZED, "unauthorized"))?;

	let content_type = req
		.headers()
		.get("content-type")
		.map_err_route((StatusCode::BAD_REQUEST, "missing content-type header"))?;
	let content_type = content_type
		.to_str()
		.map_ignore_err_route((StatusCode::BAD_REQUEST, "invalid content-type header"))?;

	let boundary = multer::parse_boundary(content_type)
		.map_ignore_err_route((StatusCode::BAD_REQUEST, "invalid content-type header"))?;

	let constraints = Constraints::new()
		.allowed_fields(vec!["metadata", "file", "captcha"])
		.size_limit(
			SizeLimit::new()
				.for_field("metadata", 30 * 1024)
				.for_field("captcha", 2048) // https://developers.cloudflare.com/turnstile/frequently-asked-questions/#what-is-the-length-of-a-turnstile-token
				.for_field("file", U::get_max_size(&global) as u64),
		);

	let mut multipart = multer::Multipart::with_constraints(req.into_body().into_stream(), boundary, constraints);

	let mut metadata = None;
	let mut file = None;
	let mut file_name = None;
	let mut file_content_type = None;
	let mut captcha = None;

	while let Some(field) = multipart
		.next_field()
		.await
		.map_ignore_err_route((StatusCode::BAD_REQUEST, "invalid multipart body"))?
	{
		let name = field
			.name()
			.map_err_route((StatusCode::BAD_REQUEST, "invalid multipart body"))?;

		match name {
			"metadata" => {
				let data = field
					.bytes()
					.await
					.map_ignore_err_route((StatusCode::BAD_REQUEST, "invalid multipart body"))?;
				metadata = Some(data);
			}
			"file" => {
				file_name = field.file_name().and_then(|s| {
					if s.len() > 125 {
						None
					} else {
						// Remove the extension
						Some(
							s.chars()
								.rev()
								.position(|c| c == '.')
								.map(|i| &s[..s.len() - i - 1])
								.unwrap_or(s)
								.to_owned(),
						)
					}
				});

				let content_type = field
					.content_type()
					.map_err_route((StatusCode::BAD_REQUEST, "invalid multipart body, missing file content-type"))?
					.as_ref();

				if !U::validate_format(&global, &auth, content_type) {
					return Err((StatusCode::BAD_REQUEST, "invalid file format").into());
				}

				file_content_type = Some(content_type.to_owned());

				let data = field
					.bytes()
					.await
					.map_ignore_err_route((StatusCode::BAD_REQUEST, "invalid multipart body"))?;
				file = Some(data);
			}
			"captcha" => {
				let data = field
					.bytes()
					.await
					.map_ignore_err_route((StatusCode::BAD_REQUEST, "invalid multipart body"))?;
				captcha = Some(data);
			}
			_ => return Err((StatusCode::BAD_REQUEST, "invalid multipart body").into()),
		}
	}

	let captcha = captcha.map_err_route((StatusCode::BAD_REQUEST, "missing captcha field"))?;
	let captcha = std::str::from_utf8(&captcha).map_ignore_err_route((StatusCode::BAD_REQUEST, "invalid captcha"))?;
	let file = file.map_err_route((StatusCode::BAD_REQUEST, "missing file field"))?;
	let file_content_type = file_content_type.map_err_route((StatusCode::BAD_REQUEST, "missing file content-type"))?;

	let metadata: U = metadata
		.map(|data| serde_json::from_slice(&data))
		.transpose()
		.map_ignore_err_route((StatusCode::BAD_REQUEST, "invalid metadata"))?
		.unwrap_or_default();

	validate_turnstile_token(&global, captcha)
		.await
		.map_err_route((StatusCode::INTERNAL_SERVER_ERROR, "failed to validate captcha"))?;

	metadata.handle(&global, auth, file_name, file, &file_content_type).await
}
