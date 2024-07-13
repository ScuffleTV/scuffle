use std::collections::HashMap;
use std::sync::Arc;

use bson::oid::ObjectId;
use scuffle_foundations::context::Context;
use scuffle_image_processor_proto::{event_callback, ErrorCode, OutputFile, OutputFormat};

use self::blocking::JobOutput;
pub use self::decoder::DecoderFrontend;
use self::resize::ResizeError;
use crate::database::Job;
use crate::drive::{Drive, DriveWriteOptions};
use crate::global::Global;

mod blocking;
mod decoder;
mod encoder;
mod frame;
mod input_download;
mod libavif;
mod libwebp;
mod resize;
mod smart_object;

#[derive(Debug, thiserror::Error)]
pub enum JobError {
	#[error("resize: {0}")]
	Resize(#[from] ResizeError),
	#[error("encoder: {0}")]
	Encoder(#[from] encoder::EncoderError),
	#[error("decoder: {0}")]
	Decoder(#[from] decoder::DecoderError),
	#[error("input download: {0}")]
	InputDownload(#[from] input_download::InputDownloadError),
	#[error("output upload: {0}")]
	OutputUpload(#[from] crate::drive::DriveError),
	#[error("mongodb: {0}")]
	Mongo(#[from] mongodb::error::Error),
	#[error("join error: {0}")]
	Join(#[from] tokio::task::JoinError),
	#[error("mismatched dimensions: {width}x{height} != {expected_width}x{expected_height}")]
	MismatchedDimensions {
		width: usize,
		height: usize,
		expected_width: usize,
		expected_height: usize,
	},
	#[error("mismatched frame count: {frame_count} != {expected_frame_count}")]
	MismatchedFrameCount {
		frame_count: usize,
		expected_frame_count: usize,
	},
	#[error("static frame index out of bounds: {idx} >= {frame_count}")]
	StaticFrameIndexOutOfBounds { idx: usize, frame_count: usize },
	#[error("invalid job")]
	InvalidJob,
	#[error("impossible output format, {0:?}, image is not animated")]
	ImpossibleOutput(OutputFormat),
	#[error("no possible outputs")]
	NoPossibleOutputs,
	#[error("{0}")]
	Internal(&'static str),
}

impl From<JobError> for scuffle_image_processor_proto::Error {
	fn from(value: JobError) -> Self {
		let message = format!("{:#}", value);

		Self {
			code: match value {
				JobError::Resize(_) => ErrorCode::Resize as i32,
				JobError::Encoder(_) => ErrorCode::Encode as i32,
				JobError::Decoder(_) => ErrorCode::Decode as i32,
				JobError::InputDownload(_) => ErrorCode::InputDownload as i32,
				JobError::Mongo(_) => ErrorCode::Internal as i32,
				JobError::Join(_) => ErrorCode::Internal as i32,
				JobError::MismatchedDimensions { .. } => ErrorCode::InvalidInput as i32,
				JobError::MismatchedFrameCount { .. } => ErrorCode::InvalidInput as i32,
				JobError::StaticFrameIndexOutOfBounds { .. } => ErrorCode::InvalidInput as i32,
				JobError::InvalidJob => ErrorCode::InvalidInput as i32,
				JobError::ImpossibleOutput(_) => ErrorCode::InvalidInput as i32,
				JobError::Internal(_) => ErrorCode::Internal as i32,
				JobError::NoPossibleOutputs => ErrorCode::InvalidInput as i32,
				JobError::OutputUpload(_) => ErrorCode::OutputUpload as i32,
			},
			message,
		}
	}
}

#[derive(Debug)]
pub struct ProcessJob {
	job: Job,
	_ctx: Context,
	permit: Arc<tokio::sync::OwnedSemaphorePermit>,
}

pub async fn spawn(job: Job, global: Arc<Global>, ctx: Context, permit: tokio::sync::OwnedSemaphorePermit) {
	let job = ProcessJob::new(job, ctx, permit);
	job.process(global).await;
}

impl ProcessJob {
	pub fn new(job: Job, ctx: Context, permit: tokio::sync::OwnedSemaphorePermit) -> Self {
		Self {
			job,
			_ctx: ctx,
			permit: Arc::new(permit),
		}
	}

	#[tracing::instrument(skip(global, self), fields(job_id = %self.job.id), name = "ProcessJob::process")]
	pub async fn process(&self, global: Arc<Global>) {
		tracing::info!("starting job");

		let start = tokio::time::Instant::now();

		crate::events::on_start(&global, &self.job).await;

		let mut future = self.process_inner(&global);
		let mut future = std::pin::pin!(future);

		let mut timeout_fut = self
			.job
			.task
			.limits
			.as_ref()
			.and_then(|l| l.max_input_duration_ms)
			.map(|timeout| Box::pin(tokio::time::sleep(std::time::Duration::from_millis(timeout as u64))));

		let result = loop {
			tokio::select! {
				_ = tokio::time::sleep(global.config().worker.refresh_interval) => {
					match self.job.refresh(&global).await {
						Ok(true) => {},
						Ok(false) => {
							tracing::warn!("lost job");
							return;
						}
						Err(err) => {
							tracing::error!("failed to refresh job: {err}");
							return;
						}
					}
				}
				Some(_) = async {
					if let Some(fut) = timeout_fut.as_mut() {
						Some(fut.await)
					} else {
						None
					}
				} => {
					tracing::warn!("timeout");
					break Err(JobError::Internal("timeout"));
				}
				result = &mut future => break result,
			}
		};

		match result {
			Ok(success) => {
				tracing::info!("job completed in {:?}", start.elapsed());
				crate::events::on_success(&global, &self.job, success).await;
			}
			Err(err) => {
				tracing::error!("failed to process job: {err}");
				crate::events::on_failure(&global, &self.job, err).await;
			}
		}

		if let Err(err) = self.job.complete(&global).await {
			tracing::error!("failed to complete job: {err}");
		}
	}

	async fn process_inner(&self, global: &Arc<Global>) -> Result<event_callback::Success, JobError> {
		let input = input_download::download_input(global, self.job.id, self.job.task.input.as_ref()).await?;
		let output = self.job.task.output.as_ref().ok_or(JobError::InvalidJob)?;

		let output_drive_path = output.drive_path.as_ref().ok_or(JobError::InvalidJob)?;

		let output_drive = global.drive(&output_drive_path.drive).ok_or(JobError::InvalidJob)?;

		let job = self.job.clone();

		let (
			decoder_info,
			JobOutput {
				output: output_results,
				input: input_metadata,
			},
		) = blocking::spawn(job.task.clone(), input, self.permit.clone()).await?;

		let is_animated = output_results.iter().any(|r| r.frame_count > 1);

		let mut files = Vec::new();

		for output_result in output_results {
			let vars = setup_vars(
				self.job.id,
				output_result.format_name.clone(),
				output_result.format,
				output_result.scale,
				output_result.width,
				output_result.height,
				output_result.format_idx,
				output_result.resize_idx,
				is_animated,
			);

			let file_path = strfmt::strfmt(&output_drive_path.path, &vars).map_err(|err| {
				tracing::error!("failed to format path: {err}");
				JobError::Internal("failed to format path")
			})?;

			let size = output_result.data.len();

			output_drive
				.write(
					&file_path,
					output_result.data.into(),
					Some(DriveWriteOptions {
						content_type: Some(content_type(output_result.format).to_owned()),
						acl: output.acl_override.clone(),
						..Default::default()
					}),
				)
				.await?;

			files.push(OutputFile {
				path: file_path,
				size: size as u32,
				format: output_result.format as i32,
				frame_count: output_result.frame_count as u32,
				height: output_result.height as u32,
				width: output_result.width as u32,
				duration_ms: output_result.duration_ms as u32,
				content_type: content_type(output_result.format).to_owned(),
				acl: output
					.acl_override
					.as_deref()
					.or(output_drive.default_acl())
					.map(|s| s.to_owned()),
			});
		}

		Ok(event_callback::Success {
			drive: output_drive_path.drive.clone(),
			input_metadata: Some(input_metadata),
			files,
		})
	}
}

fn setup_vars(
	id: ObjectId,
	format_name: Option<String>,
	format: OutputFormat,
	scale: Option<usize>,
	width: usize,
	height: usize,
	format_idx: usize,
	resize_idx: usize,
	is_animated: bool,
) -> HashMap<String, String> {
	let format_name = format_name.unwrap_or_else(|| match format {
		OutputFormat::AvifAnim => "avif_anim".to_owned(),
		OutputFormat::AvifStatic => "avif_static".to_owned(),
		OutputFormat::WebpAnim => "webp_anim".to_owned(),
		OutputFormat::WebpStatic => "webp_static".to_owned(),
		OutputFormat::PngStatic => "png_static".to_owned(),
		OutputFormat::GifAnim => "gif_anim".to_owned(),
	});

	let scale = scale.map(|scale| scale.to_string()).unwrap_or_else(|| "".to_owned());

	let static_ = match format {
		OutputFormat::AvifStatic | OutputFormat::PngStatic | OutputFormat::WebpStatic if is_animated => "_static",
		_ => "",
	};

	let ext = match format {
		OutputFormat::AvifAnim | OutputFormat::AvifStatic => "avif",
		OutputFormat::PngStatic => "png",
		OutputFormat::WebpAnim | OutputFormat::WebpStatic => "webp",
		OutputFormat::GifAnim => "gif",
	};

	[
		("id".to_owned(), id.to_string()),
		("format".to_owned(), format_name),
		("scale".to_owned(), scale),
		("width".to_owned(), width.to_string()),
		("height".to_owned(), height.to_string()),
		("format_idx".to_owned(), format_idx.to_string()),
		("resize_idx".to_owned(), resize_idx.to_string()),
		("static".to_owned(), static_.to_owned()),
		("ext".to_owned(), ext.to_owned()),
	]
	.into_iter()
	.collect::<HashMap<_, _>>()
}

fn content_type(format: OutputFormat) -> &'static str {
	match format {
		OutputFormat::AvifAnim | OutputFormat::AvifStatic => "image/avif",
		OutputFormat::WebpAnim | OutputFormat::WebpStatic => "image/webp",
		OutputFormat::PngStatic => "image/png",
		OutputFormat::GifAnim => "image/gif",
	}
}
