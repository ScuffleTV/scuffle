use std::borrow::Cow;
use std::sync::Arc;

use bytes::Bytes;
use common::task::AsyncTask;
use file_format::FileFormat;
use futures::FutureExt;
use prost::Message;
use tokio::select;
use tokio::sync::SemaphorePermit;

use self::decoder::DecoderBackend;
use super::error::{ProcessorError, Result};
use super::utils;
use crate::database;
use crate::global::ImageProcessorGlobal;
use crate::processor::utils::refresh_job;

pub(crate) mod decoder;
pub(crate) mod encoder;
pub(crate) mod frame;
pub(crate) mod libavif;
pub(crate) mod libwebp;
pub(crate) mod process;
pub(crate) mod resize;
pub(crate) mod smart_object;

pub(crate) struct Job<'a, G: ImageProcessorGlobal> {
	pub(crate) global: &'a Arc<G>,
	pub(crate) job: database::Job,
}

pub async fn handle_job(
	global: &Arc<impl ImageProcessorGlobal>,
	_ticket: SemaphorePermit<'_>,
	job: database::Job,
) -> Result<()> {
	let job = Job { global, job };
	job.process().await
}

impl<'a, G: ImageProcessorGlobal> Job<'a, G> {
	async fn download_source(&self) -> Result<Bytes> {
		if self.job.task.input_path.starts_with("http://") || self.job.task.input_path.starts_with("https://") {
			if !self.global.config().allow_http {
				return Err(ProcessorError::HttpDownloadDisabled);
			}

			tracing::info!("downloading {}", self.job.task.input_path);

			let response = reqwest::get(&self.job.task.input_path)
				.await
				.map_err(ProcessorError::HttpDownload)?;

			response.error_for_status_ref().map_err(ProcessorError::HttpDownload)?;

			Ok(response.bytes().await.map_err(ProcessorError::HttpDownload)?)
		} else {
			tracing::info!(
				"downloading {}/{}",
				self.global.config().source_bucket.name,
				self.job.task.input_path
			);

			let response = self
				.global
				.s3_source_bucket()
				.get_object(&self.job.task.input_path)
				.await
				.map_err(ProcessorError::S3Download)?;

			if (200..299).contains(&response.status_code()) {
				Ok(response.bytes().clone())
			} else {
				Err(ProcessorError::S3Download(s3::error::S3Error::HttpFail))
			}
		}
	}

	pub(crate) async fn process(self) -> Result<()> {
		if let Err(e) = self.process_with_timeout().await {
			tracing::warn!(err = %e, "job failed");
			tracing::debug!("publishing job failure event to {}", self.job.task.callback_subject);
			self.global
				.jetstream()
				.publish(
					self.job.task.callback_subject.clone(),
					pb::scuffle::platform::internal::events::ProcessedImage {
						job_id: Some(self.job.id.0.into()),
						result: Some(pb::scuffle::platform::internal::events::processed_image::Result::Failure(
							pb::scuffle::platform::internal::events::processed_image::Failure {
								reason: format!("{}", e),
							},
						)),
					}
					.encode_to_vec()
					.into(),
				)
				.await
				.map_err(|e| {
					tracing::error!(err = %e, "failed to publish event");
					e
				})?;
		}

		// delete job
		utils::delete_job(&self.global, self.job.id.0).await?;

		Ok(())
	}

	async fn process_with_timeout(&self) -> Result<()> {
		let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));

		let job_id = self.job.id.0;
		let max_processing_time_ms = self.job.task.limits.as_ref().map(|l| l.max_processing_time_ms);

		let time_limit = async {
			if let Some(max_processing_time_ms) = max_processing_time_ms {
				tokio::time::sleep(std::time::Duration::from_millis(max_processing_time_ms as u64)).await;
				Err(ProcessorError::TimeLimitExceeded)
			} else {
				Ok(())
			}
		};

		let global = self.global.clone();
		let mut process = std::pin::pin!(self.inner_process());
		let time_limit = std::pin::pin!(time_limit);
		let mut time_limit = time_limit.fuse();

		loop {
			select! {
				_ = interval.tick() => {
					refresh_job(&global, job_id).await?;
				},
				Err(e) = &mut time_limit => {
					return Err(e);
				},
				r = &mut process => {
					return r;
				},
			}
		}
	}

	async fn inner_process(&self) -> Result<()> {
		let input_data = self.download_source().await?;

		let backend = DecoderBackend::from_format(FileFormat::from_bytes(&input_data))?;

		let url_prefix = format!("result/{}{}", self.job.task.output_prefix, self.job.id);

		let job_c = self.job.clone();

		tracing::info!("processing job");

		let images = AsyncTask::spawn_blocking("process", move || {
			process::process_job(backend, &job_c, Cow::Borrowed(&input_data))
		})
		.join()
		.await
		.map_err(|e| {
			tracing::error!(err = %e, "failed to process job");
			ProcessorError::BlockingTaskSpawn
		})??;

		for image in images.images.iter() {
			// image upload
			let url = image.url(&url_prefix);
			tracing::info!("uploading result to {}/{}", self.global.config().target_bucket.name, url);
			let resp = self
				.global
				.s3_target_bucket()
				.put_object_with_content_type(url, &image.data, image.content_type())
				.await
				.map_err(ProcessorError::S3Upload)?;

			if !(200..299).contains(&resp.status_code()) {
				return Err(ProcessorError::S3Upload(s3::error::S3Error::HttpFail));
			}
		}
		// job completion
		tracing::debug!("publishing job completion event to {}", self.job.task.callback_subject);
		self.global
			.jetstream()
			.publish(
				self.job.task.callback_subject.clone(),
				pb::scuffle::platform::internal::events::ProcessedImage {
					job_id: Some(self.job.id.0.into()),
					result: Some(pb::scuffle::platform::internal::events::processed_image::Result::Success(
						pb::scuffle::platform::internal::events::processed_image::Success {
							variants: images
								.images
								.iter()
								.map(|i| pb::scuffle::platform::internal::types::ProcessedImageVariant {
									path: i.url(&url_prefix),
									format: i.request.1.into(),
									width: i.width as u32,
									height: i.height as u32,
									byte_size: i.data.len() as u32,
									scale: i.request.0 as u32,
								})
								.collect(),
						},
					)),
				}
				.encode_to_vec()
				.into(),
			)
			.await
			.map_err(|e| {
				tracing::error!(err = %e, "failed to publish event");
				e
			})?;

		Ok(())
	}
}
