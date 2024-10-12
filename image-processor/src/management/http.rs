use scuffle_foundations::http::server::axum::extract::State;
use scuffle_foundations::http::server::axum::routing::post;
use scuffle_foundations::http::server::axum::{Json, Router};
use scuffle_image_processor_proto::{
	CancelTaskRequest, CancelTaskResponse, ErrorCode, ProcessImageRequest, ProcessImageResponse,
};

use super::ManagementServer;

impl ManagementServer {
	#[tracing::instrument(skip_all)]
	pub async fn run_http(&self, addr: std::net::SocketAddr) -> Result<(), scuffle_foundations::http::server::Error> {
		let router = Router::new()
			.route("/process_image", post(process_image))
			.route("/cancel_task", post(cancel_task))
			.fallback(not_found)
			.with_state(self.clone());

		tracing::info!("HTTP management server listening on {}", addr);

		scuffle_foundations::http::server::Server::builder()
			.bind(addr)
			.build(router)?
			.start_and_wait()
			.await
	}
}

#[tracing::instrument(skip_all)]
async fn not_found() -> (http::StatusCode, &'static str) {
	(http::StatusCode::NOT_FOUND, "Not Found")
}

#[tracing::instrument(skip_all)]
async fn process_image(
	State(server): State<ManagementServer>,
	Json(request): Json<ProcessImageRequest>,
) -> (http::StatusCode, Json<ProcessImageResponse>) {
	let resp = match server.process_image(request).await {
		Ok(resp) => resp,
		Err(err) => ProcessImageResponse {
			id: "".to_owned(),
			upload_info: None,
			error: Some(err),
		},
	};

	let status = resp
		.error
		.as_ref()
		.map_or(http::StatusCode::OK, |err| map_error_code(err.code()));
	(status, Json(resp))
}

#[tracing::instrument(skip_all)]
async fn cancel_task(
	State(server): State<ManagementServer>,
	Json(request): Json<CancelTaskRequest>,
) -> (http::StatusCode, Json<CancelTaskResponse>) {
	let resp = match server.cancel_task(request).await {
		Ok(resp) => resp,
		Err(err) => CancelTaskResponse { error: Some(err) },
	};

	let status = resp
		.error
		.as_ref()
		.map_or(http::StatusCode::OK, |err| map_error_code(err.code()));
	(status, Json(resp))
}

fn map_error_code(code: ErrorCode) -> http::StatusCode {
	match code {
		ErrorCode::InvalidInput => http::StatusCode::BAD_REQUEST,
		ErrorCode::Internal => http::StatusCode::INTERNAL_SERVER_ERROR,
		ErrorCode::NotImplemented => http::StatusCode::NOT_IMPLEMENTED,
		ErrorCode::Decode => http::StatusCode::INTERNAL_SERVER_ERROR,
		ErrorCode::Encode => http::StatusCode::INTERNAL_SERVER_ERROR,
		ErrorCode::InputDownload => http::StatusCode::INTERNAL_SERVER_ERROR,
		ErrorCode::OutputUpload => http::StatusCode::INTERNAL_SERVER_ERROR,
		ErrorCode::Resize => http::StatusCode::INTERNAL_SERVER_ERROR,
	}
}
