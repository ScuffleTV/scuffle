use std::borrow::Cow;
use std::sync::Arc;
use std::time::Duration;

use scuffle_utils::prelude::FutureTimeout;
use scuffle_utils::task::AsyncTask;
use aws_sdk_s3::types::ObjectCannedAcl;
use bytes::Bytes;
use file_format::FileFormat;
use futures::FutureExt;
use prost::Message;
use tokio::select;
use tracing::Instrument;

use self::decoder::DecoderBackend;
use super::error::{ProcessorError, Result};
use super::utils;
use crate::{database, pb};
use crate::global::ImageProcessorGlobal;
use crate::processor::utils::refresh_job;

pub(crate) mod decoder;
pub(crate) mod encoder;
pub(crate) mod frame;
pub(crate) mod libavif;
pub(crate) mod libwebp;
pub(crate) mod process;
pub(crate) mod resize;
pub(crate) mod scaling;
pub(crate) mod smart_object;

pub(crate) struct Job<'a, G: ImageProcessorGlobal> {
	pub(crate) global: &'a Arc<G>,
	pub(crate) job: database::Job,
}

#[tracing::instrument(skip(global, job), fields(job_id = %job.id), level = "info")]
pub async fn handle_job(global: &Arc<impl ImageProcessorGlobal>, job: database::Job) {
	let job = Job { global, job };

	tracing::info!("processing job");

	if let Err(err) = job.process().in_current_span().await {
		tracing::error!(err = %err, "job failed");
	}
}

impl<'a, G: ImageProcessorGlobal> Job<'a, G> {
	async fn download_source(&self) -> Result<Bytes> {
		if self.job.task.input_path.starts_with("http://") || self.job.task.input_path.starts_with("https://") {
			if !self.global.config().allow_http {
				return Err(ProcessorError::HttpDownloadDisabled);
			}

			tracing::debug!("downloading {}", self.job.task.input_path);

			Ok(self
				.global
				.http_client()
				.get(&self.job.task.input_path)
				.send()
				.await
				.map_err(ProcessorError::HttpDownload)?
				.error_for_status()
				.map_err(ProcessorError::HttpDownload)?
				.bytes()
				.await
				.map_err(ProcessorError::HttpDownload)?)
		} else {
			tracing::debug!(
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

			let body = response.body.collect().await.map_err(ProcessorError::S3DownloadStream)?;
			Ok(body.into_bytes())
		}
	}

	pub(crate) async fn process(self) -> Result<()> {
		if let Err(e) = self.process_with_timeout().in_current_span().await {
			tracing::warn!(err = %e, "job failed");
			tracing::debug!("publishing job failure event to {}", self.job.task.callback_subject);
			self.global
				.nats()
				.publish(
					self.job.task.callback_subject.clone(),
					pb::EventPayload {
						id: todo!(),
					}
					.encode_to_vec()
					.into(),
				)
				.in_current_span()
				.await
				.map_err(|e| {
					tracing::error!(err = %e, "failed to publish event");
					e
				})?;
		}

		// delete job
		utils::delete_job(self.global, self.job.id).await?;

		Ok(())
	}

	async fn process_with_timeout(&self) -> Result<()> {
		let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));

		let job_id = self.job.id;
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
		let mut process = std::pin::pin!(self.inner_process().in_current_span());
		let time_limit = std::pin::pin!(time_limit);
		let mut time_limit = time_limit.fuse();

		loop {
			select! {
				_ = interval.tick() => {
					refresh_job(&global, job_id).in_current_span().await?;
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
		let input_data = {
			let mut tries = 0;
			loop {
				match self.download_source().timeout(Duration::from_secs(5)).await {
					Ok(Ok(data)) => break data,
					Ok(Err(e)) => {
						if tries >= 60 {
							return Err(e);
						}

						tries += 1;
						tracing::debug!(err = %e, "failed to download source, retrying");
						tokio::time::sleep(std::time::Duration::from_secs(1)).await;
					}
					Err(_) => {
						if tries >= 60 {
							return Err(ProcessorError::DownloadTimeout);
						}

						tries += 1;
						tracing::debug!("download timed out, retrying");
					}
				}
			}
		};

		let backend = DecoderBackend::from_format(FileFormat::from_bytes(&input_data))?;

		let job_c = self.job.clone();

		tracing::debug!("processing job");

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
			let url = image.url(&self.job.task.output_prefix);
			// image upload
			tracing::debug!("uploading result to {}/{}", self.global.config().target_bucket.name, url);
			self.global
				.s3_target_bucket()
				.put_object(
					url,
					image.data.clone(),
					Some(PutObjectOptions {
						acl: Some(ObjectCannedAcl::PublicRead),
						content_type: Some(image.content_type().into()),
					}),
				)
				.in_current_span()
				.await
				.map_err(ProcessorError::S3Upload)?;
		}
		// job completion
		tracing::debug!("publishing job completion event to {}", self.job.task.callback_subject);
		self.global
			.nats()
			.publish(
				self.job.task.callback_subject.clone(),
				pb::EventPayload {
					id: todo!(),
				}
				.encode_to_vec()
				.into(),
			)
			.in_current_span()
			.await
			.map_err(|e| {
				tracing::error!(err = %e, "failed to publish event");
				e
			})?;

		tracing::info!("job completed");

		Ok(())
	}
}
