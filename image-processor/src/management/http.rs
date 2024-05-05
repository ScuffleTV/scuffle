use scuffle_foundations::http::server::axum::{extract::State, Json, Router, routing::post};
use scuffle_image_processor_proto::{CancelTaskRequest, CancelTaskResponse, ErrorCode, ProcessImageRequest, ProcessImageResponse};

use super::ManagementServer;

impl ManagementServer {
	pub async fn run_http(&self) -> Result<(), scuffle_foundations::http::server::Error> {
        let router = Router::new()
            .route("/process_image", post(process_image))
            .route("/cancel_task", post(cancel_task))
            .with_state(self.clone());

        let addr = self.global.config().management.http.bind;
        scuffle_foundations::http::server::Server::builder()
            .bind(addr)
            .build(router)?
            .start_and_wait()
            .await
    }
}

async fn process_image(
    State(server): State<ManagementServer>,
    Json(request): Json<ProcessImageRequest>,
) -> (http::StatusCode, Json<ProcessImageResponse>) {
    let resp = match server.process_image(request).await {
        Ok(resp) => resp,
        Err(err) => ProcessImageResponse {
            id: "".to_owned(),
            error: Some(err),
        }
    };

    let status = resp.error.as_ref().map_or(http::StatusCode::OK, |err| map_error_code(err.code()));
    (status, Json(resp))
}

async fn cancel_task(
    State(server): State<ManagementServer>,
    Json(request): Json<CancelTaskRequest>,
) -> (http::StatusCode, Json<CancelTaskResponse>) {
    let resp = match server.cancel_task(request).await {
        Ok(resp) => resp,
        Err(err) => CancelTaskResponse {
            error: Some(err),
        }
    };

    let status = resp.error.as_ref().map_or(http::StatusCode::OK, |err| map_error_code(err.code()));
    (status, Json(resp))
}

fn map_error_code(code: ErrorCode) -> http::StatusCode {
    match code {
        ErrorCode::InvalidInput => http::StatusCode::BAD_REQUEST,
        ErrorCode::InternalError => http::StatusCode::INTERNAL_SERVER_ERROR,
        ErrorCode::Unknown => http::StatusCode::INTERNAL_SERVER_ERROR,
    }
}