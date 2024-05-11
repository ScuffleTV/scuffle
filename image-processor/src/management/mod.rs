use std::sync::Arc;

use anyhow::Context;
use bytes::Bytes;
use scuffle_image_processor_proto::{
	CancelTaskRequest, CancelTaskResponse, Error, ErrorCode, ProcessImageRequest, ProcessImageResponse,
};

use crate::database::Job;
use crate::drive::{Drive, DriveWriteOptions};
use crate::global::Global;
use crate::management::validation::{validate_input_upload, validate_task, FragmentBuf};

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
		if let Some(input_upload) = request.input_upload.as_ref() {
			validate_input_upload(&self.global, fragment.push("input_upload"), Some(input_upload))?;
		}

		if let Some(input_upload) = request.input_upload {
			let drive_path = input_upload.path.unwrap();
			let drive = self.global.drive(&drive_path.drive).unwrap();

			drive
				.write(
					&drive_path.path,
					Bytes::from(input_upload.binary),
					Some(DriveWriteOptions {
						acl: input_upload.acl,
						cache_control: input_upload.cache_control,
						content_disposition: input_upload.content_disposition,
						content_type: input_upload.content_type,
					}),
				)
				.await
				.map_err(|err| {
					tracing::error!("failed to write input upload: {:#}", err);
					Error {
						code: ErrorCode::Internal as i32,
						message: format!("failed to write input upload: {err}"),
					}
				})?;
		}

		let job = Job::new(&self.global, request.task.unwrap(), request.priority, request.ttl)
			.await
			.map_err(|err| {
				tracing::error!("failed to create job: {:#}", err);
				Error {
					code: ErrorCode::Internal as i32,
					message: format!("failed to create job: {err}"),
				}
			})?;

		Ok(ProcessImageResponse {
			id: job.id.to_string(),
			error: None,
		})
	}

	async fn cancel_task(&self, request: CancelTaskRequest) -> Result<CancelTaskResponse, Error> {
		match Job::cancel(
			&self.global,
			request.id.parse().map_err(|err| Error {
				code: ErrorCode::InvalidInput as i32,
				message: format!("id: {err}"),
			})?,
		)
		.await
		{
			Ok(Some(_)) => Ok(CancelTaskResponse { error: None }),
			Ok(None) => Err(Error {
				code: ErrorCode::InvalidInput as i32,
				message: "not found".to_owned(),
			}),
			Err(err) => {
				tracing::error!("failed to cancel job: {:#}", err);
				Err(Error {
					code: ErrorCode::Internal as i32,
					message: format!("failed to cancel job: {err}"),
				})
			}
		}
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
