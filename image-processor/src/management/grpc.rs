use scuffle_image_processor_proto::{CancelTaskRequest, CancelTaskResponse, ProcessImageRequest, ProcessImageResponse};
use tonic::{Request, Response};

use super::ManagementServer;

impl ManagementServer {
	pub async fn run_grpc(&self) -> Result<(), tonic::transport::Error> {
		let addr = self.global.config().management.grpc.bind;
		let server = tonic::transport::Server::builder()
			.add_service(scuffle_image_processor_proto::image_processor_server::ImageProcessorServer::new(self.clone()))
			.serve_with_shutdown(addr, scuffle_foundations::context::Context::global().into_done());

		tracing::info!(%addr, "gRPC server listening");
		server.await
	}
}

#[async_trait::async_trait]
impl scuffle_image_processor_proto::image_processor_server::ImageProcessor for ManagementServer {
	async fn process_image(&self, request: Request<ProcessImageRequest>) -> tonic::Result<Response<ProcessImageResponse>> {
		let resp = match self.process_image(request.into_inner()).await {
			Ok(resp) => resp,
			Err(err) => ProcessImageResponse {
				id: "".to_owned(),
				error: Some(err),
			},
		};

		Ok(Response::new(resp))
	}

	async fn cancel_task(&self, request: Request<CancelTaskRequest>) -> tonic::Result<Response<CancelTaskResponse>> {
		let resp = match self.cancel_task(request.into_inner()).await {
			Ok(resp) => resp,
			Err(err) => CancelTaskResponse { error: Some(err) },
		};

		Ok(Response::new(resp))
	}
}
