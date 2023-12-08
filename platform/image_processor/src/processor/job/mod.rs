use std::path::{Path, PathBuf};
use std::sync::Arc;

use file_format::FileFormat;
use tokio::select;
use tokio::sync::SemaphorePermit;

use self::decoder::DecoderBackend;
use super::error::{ProcessorError, Result};
use crate::database;
use crate::global::ImageProcessorGlobal;
use crate::processor::utils::refresh_job;

pub(crate) mod decoder;
pub(crate) mod encoder;
pub(crate) mod frame;
pub(crate) mod frame_deduplicator;
pub(crate) mod libavif;
pub(crate) mod libwebp;
pub(crate) mod process;
pub(crate) mod resize;
pub(crate) mod smart_object;

pub(crate) struct Job<'a, G: ImageProcessorGlobal> {
	pub(crate) global: &'a Arc<G>,
	pub(crate) job: database::Job,
	pub(crate) working_directory: std::path::PathBuf,
}

async fn handle_error(err: ProcessorError) -> Result<()> {
	Err(err)
}

pub async fn handle_job(
	global: &Arc<impl ImageProcessorGlobal>,
	parent_directory: &Path,
	_ticket: SemaphorePermit<'_>,
	job: database::Job,
) -> Result<()> {
	let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));

	let job_id = job.id.0;
	let max_processing_time_ms = job.task.limits.as_ref().map(|l| l.max_processing_time_ms);

	let working_directory = parent_directory.join(job_id.to_string());

	let job = Job {
		global,
		job,
		working_directory,
	};

	let time_limit = async {
		if let Some(max_processing_time_ms) = max_processing_time_ms {
			tokio::time::sleep(std::time::Duration::from_millis(max_processing_time_ms as u64)).await;
			Err(ProcessorError::TimeLimitExceeded)
		} else {
			Ok(())
		}
	};

	let mut process = std::pin::pin!(job.process());
	let mut time_limit = std::pin::pin!(time_limit);

	loop {
		select! {
			_ = interval.tick() => {
				refresh_job(global, job_id).await?;
			},
			Err(e) = &mut time_limit => {
				return handle_error(e).await;
			},
			r = &mut process => {
				return if let Err(e) = r {
					handle_error(e).await
				} else {
					Ok(())
				};
			},
		}
	}
}

impl<'a, G: ImageProcessorGlobal> Job<'a, G> {
	async fn download_source(&self) -> Result<PathBuf> {
		let dest = self.working_directory.join("input");

		let mut fs = tokio::fs::OpenOptions::new()
			.write(true)
			.create_new(true)
			.open(&dest)
			.await
			.map_err(ProcessorError::FileCreate)?;

		self.global
			.s3_source_bucket()
			.get_object_to_writer(&self.job.id.to_string(), &mut fs)
			.await
			.map_err(ProcessorError::S3Download)?;

		Ok(dest)
	}

	pub(crate) async fn process(self) -> Result<()> {
		let input_path = self.download_source().await?;

		let backend = DecoderBackend::from_format(FileFormat::from_file(&input_path).map_err(ProcessorError::FileFormat)?)?;

		let output_path = self.working_directory.join("frames");
		tokio::fs::create_dir_all(&output_path)
			.await
			.map_err(ProcessorError::DirectoryCreate)?;

		let job = self.job.clone();

		let images = tokio::task::spawn_blocking(move || process::process_job(backend, &input_path, &job))
			.await
			.unwrap_or_else(|e| {
				tracing::error!(error = %e, "failed to spawn blocking task");
				Err(ProcessorError::BlockingTaskSpawn)
			})?;

		dbg!(&images);

		// TODO: handle the image upload, and the job completion
		todo!("handle the image upload, and the job completion");
	}
}

impl<'a, G: ImageProcessorGlobal> Drop for Job<'a, G> {
	fn drop(&mut self) {
		let working_directory = self.working_directory.clone();
		tokio::spawn(async move {
			tokio::fs::remove_dir_all(working_directory).await.unwrap_or_else(|e| {
				tracing::error!(error = %e, "failed to remove working directory");
			});
		});
	}
}
