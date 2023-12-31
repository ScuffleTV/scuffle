use std::collections::HashSet;
use std::path::Path;

use pb::scuffle::platform::internal::types::ImageFormat;

use super::decoder::{Decoder, DecoderBackend, LoopCount};
use super::encoder::{AnyEncoder, Encoder, EncoderFrontend, EncoderSettings};
use super::frame::FrameOwned;
use super::frame_deduplicator::FrameDeduplicator;
use super::resize::{ImageResizer, ImageResizerTarget};
use crate::database::Job;
use crate::processor::error::{ProcessorError, Result};

#[derive(Debug)]
pub struct Image {
	pub width: usize,
	pub height: usize,
	pub frame_count: usize,
	pub duration: f64,
	pub encoder: EncoderFrontend,
	pub data: Vec<u8>,
	pub loop_count: LoopCount,
	pub request: (usize, ImageFormat),
}

impl Image {
	pub fn file_extension(&self) -> &'static str {
		match self.request.1 {
			ImageFormat::Avif | ImageFormat::AvifStatic => "avif",
			ImageFormat::Webp | ImageFormat::WebpStatic => "webp",
			ImageFormat::Gif => "gif",
			ImageFormat::PngStatic => "png",
		}
	}

	pub fn content_type(&self) -> &'static str {
		match self.request.1 {
			ImageFormat::Avif | ImageFormat::AvifStatic => "image/avif",
			ImageFormat::Webp | ImageFormat::WebpStatic => "image/webp",
			ImageFormat::Gif => "image/gif",
			ImageFormat::PngStatic => "image/png",
		}
	}

	pub fn is_static(&self) -> bool {
		matches!(
			self.request.1,
			ImageFormat::AvifStatic | ImageFormat::WebpStatic | ImageFormat::PngStatic
		)
	}

	pub fn url(&self, prefix: &str) -> String {
		format!(
			"{}/{}{}x.{}",
			prefix,
			self.is_static().then_some("static_").unwrap_or_default(),
			self.request.0,
			self.file_extension()
		)
	}
}

#[derive(Debug)]
pub struct Images {
	pub images: Vec<Image>,
}

struct ResizerStack {
	resizer: ImageResizer,
	first_frame: Option<FrameOwned>,
	frame_count: usize,
	scale: usize,
	static_encoders: Vec<AnyEncoder>,
	animation_encoders: Vec<AnyEncoder>,
}

pub fn process_job(backend: DecoderBackend, input_path: &Path, job: &Job) -> Result<Images> {
	let mut decoder = backend.build(input_path, job)?;

	let info = decoder.info();

	let formats = job.task.formats().collect::<HashSet<_>>();
	let scales = job.task.scales.iter().map(|s| *s as usize).collect::<HashSet<_>>();

	if formats.is_empty() || scales.is_empty() {
		tracing::debug!("no formats or scales specified");
		return Err(ProcessorError::InvalidJobState);
	}

	let static_formats = formats
		.iter()
		.filter_map(|f| match f {
			ImageFormat::AvifStatic => Some(EncoderFrontend::LibAvif),
			ImageFormat::WebpStatic => Some(EncoderFrontend::LibWebp),
			ImageFormat::PngStatic => Some(EncoderFrontend::Png),
			_ => None,
		})
		.collect::<Vec<_>>();

	let animation_formats = formats
		.iter()
		.filter_map(|f| match f {
			ImageFormat::Avif => Some(EncoderFrontend::LibAvif),
			ImageFormat::Webp => Some(EncoderFrontend::LibWebp),
			ImageFormat::Gif => Some(EncoderFrontend::Gifski),
			_ => None,
		})
		.collect::<Vec<_>>();

	if static_formats.is_empty() && animation_formats.is_empty() {
		tracing::debug!("no static or animation formats specified");
		return Err(ProcessorError::InvalidJobState);
	}

	let anim_settings = EncoderSettings {
		fast: true,
		loop_count: info.loop_count,
		timescale: info.timescale,
		static_image: false,
	};

	let static_settings = EncoderSettings {
		fast: true,
		loop_count: info.loop_count,
		timescale: info.timescale,
		static_image: true,
	};

	let (base_width, base_height) = if job.task.upscale {
		(job.task.base_width as f64, job.task.base_height as f64)
	} else {
		let largest_scale = scales.iter().max().copied().unwrap_or(1);

		let width = info.width as f64 / largest_scale as f64;
		let height = info.height as f64 / largest_scale as f64;

		if width > job.task.base_width as f64 && height > job.task.base_height as f64 {
			(width, height)
		} else {
			(job.task.base_width as f64, job.task.base_height as f64)
		}
	};

	let mut resizers = scales
		.into_iter()
		.map(|scale| {
			Ok::<_, ProcessorError>(ResizerStack {
				resizer: ImageResizer::new(ImageResizerTarget {
					height: base_height.ceil() as usize * scale,
					width: base_width.ceil() as usize * scale,
					algorithm: job.task.resize_algorithm(),
					method: job.task.resize_method(),
				}),
				animation_encoders: if info.frame_count > 1 {
					animation_formats
						.iter()
						.map(|&frontend| frontend.build(anim_settings))
						.collect::<Result<Vec<_>, _>>()?
				} else {
					Vec::new()
				},
				scale,
				static_encoders: static_formats
					.iter()
					.map(|&frontend| frontend.build(static_settings))
					.collect::<Result<Vec<_>, _>>()?,
				first_frame: None,
				frame_count: 0,
			})
		})
		.collect::<Result<Vec<_>, _>>()?;

	let mut deduplicator = FrameDeduplicator::new();

	loop {
		let frame = match decoder.decode()? {
			Some(frame) => match deduplicator.deduplicate(frame.as_ref()) {
				Some(frame) => frame,
				None => continue,
			},
			None => match deduplicator.flush() {
				Some(frame) => frame,
				None => break,
			},
		};

		for stack in resizers.iter_mut() {
			let frame = stack.resizer.resize(frame.as_ref())?;
			stack.frame_count += 1;
			if stack.frame_count != 1 {
				if let Some(first_frame) = stack.first_frame.take() {
					for encoder in stack.animation_encoders.iter_mut() {
						encoder.add_frame(first_frame.as_ref())?;
					}
				}

				for encoder in stack.animation_encoders.iter_mut() {
					encoder.add_frame(frame.as_ref())?;
				}
			} else {
				for encoder in stack.static_encoders.iter_mut() {
					encoder.add_frame(frame.as_ref())?;
				}

				stack.first_frame = Some(frame.into_owned());
			}
		}
	}

	let mut images = Vec::new();

	for stack in resizers.into_iter() {
		for encoder in stack.animation_encoders.into_iter() {
			let info = encoder.info();
			let output = encoder.finish()?;
			images.push(Image {
				width: info.width,
				height: info.height,
				frame_count: info.frame_count,
				duration: info.duration as f64 / info.timescale as f64,
				encoder: info.frontend,
				data: output,
				loop_count: info.loop_count,
				request: (
					stack.scale,
					match info.frontend {
						EncoderFrontend::Gifski => ImageFormat::Gif,
						EncoderFrontend::LibAvif => ImageFormat::Avif,
						EncoderFrontend::LibWebp => ImageFormat::Webp,
						EncoderFrontend::Png => unreachable!(),
					},
				),
			});
		}

		for encoder in stack.static_encoders.into_iter() {
			let info = encoder.info();
			let output = encoder.finish()?;
			images.push(Image {
				width: info.width,
				height: info.height,
				frame_count: info.frame_count,
				duration: info.duration as f64 / info.timescale as f64,
				encoder: info.frontend,
				data: output,
				loop_count: info.loop_count,
				request: (
					stack.scale,
					match info.frontend {
						EncoderFrontend::LibAvif => ImageFormat::AvifStatic,
						EncoderFrontend::LibWebp => ImageFormat::WebpStatic,
						EncoderFrontend::Png => ImageFormat::PngStatic,
						EncoderFrontend::Gifski => unreachable!(),
					},
				),
			});
		}
	}

	Ok(Images { images })
}
