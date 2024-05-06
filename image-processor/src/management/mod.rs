use std::sync::Arc;

use anyhow::Context;
use scuffle_image_processor_proto::{
	CancelTaskRequest, CancelTaskResponse, Error, ErrorCode, ProcessImageRequest, ProcessImageResponse,
};
use url::Url;

use crate::global::Global;
use crate::management::validation::{validate_task, Fragment, FragmentBuf};

pub mod grpc;
pub mod http;

mod validation;

#[derive(Clone)]
struct ManagementServer {
	global: Arc<Global>,
}

impl ManagementServer {
	async fn process_image(&self, request: ProcessImageRequest) -> Result<ProcessImageResponse, Error> {
		let mut fragment = FragmentBuf::new();

		validate_task(&self.global, fragment.push("task"), request.task.as_ref())?;

		// We need to do validation here.
		if let Some(input_upload) = request.input_upload.as_ref() {}

		todo!()
	}

	async fn cancel_task(&self, request: CancelTaskRequest) -> Result<CancelTaskResponse, Error> {
		todo!()
	}
}

pub async fn start(global: Arc<Global>) -> anyhow::Result<()> {
	let server = ManagementServer { global };

	let http = async {
		if server.global.config().management.http.enabled {
			server.run_http().await.context("http")
		} else {
			Ok(())
		}
	};
	let grpc = async {
		if server.global.config().management.grpc.enabled {
			server.run_grpc().await.context("grpc")
		} else {
			Ok(())
		}
	};

	futures::future::try_join(http, grpc).await.context("management")?;

	Ok(())
}
