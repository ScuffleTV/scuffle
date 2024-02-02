use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use bytes::Bytes;
use pb::scuffle::platform::internal::image_processor::task;
use pb::scuffle::platform::internal::types::ImageFormat;
use rgb::ComponentBytes;
use sha2::Digest;

use super::decoder::{Decoder, DecoderBackend, LoopCount};
use super::encoder::{AnyEncoder, Encoder, EncoderFrontend, EncoderSettings};
use super::resize::{ImageResizer, ImageResizerTarget};
use crate::database::Job;
use crate::processor::error::{ProcessorError, Result};
use crate::processor::job::scaling::{Ratio, ScalingOptions};

#[derive(Debug)]
#[allow(dead_code)]
pub struct Image {
	pub width: usize,
	pub height: usize,
	pub frame_count: usize,
	pub duration: f64,
	pub encoder: EncoderFrontend,
	pub data: Bytes,
	pub loop_count: LoopCount,
	pub request: ImageFormat,
}

impl Image {
	pub fn file_extension(&self) -> &'static str {
		match self.request {
			ImageFormat::Avif | ImageFormat::AvifStatic => "avif",
			ImageFormat::Webp | ImageFormat::WebpStatic => "webp",
			ImageFormat::Gif => "gif",
			ImageFormat::PngStatic => "png",
		}
	}

	pub fn content_type(&self) -> &'static str {
		match self.request {
			ImageFormat::Avif | ImageFormat::AvifStatic => "image/avif",
			ImageFormat::Webp | ImageFormat::WebpStatic => "image/webp",
			ImageFormat::Gif => "image/gif",
			ImageFormat::PngStatic => "image/png",
		}
	}

	pub fn is_static(&self) -> bool {
		matches!(
			self.request,
			ImageFormat::AvifStatic | ImageFormat::WebpStatic | ImageFormat::PngStatic
		)
	}

	pub fn url(&self, prefix: &str) -> String {
		format!(
			"{prefix}/{static_prefix}{width}x{height}.{ext}",
			prefix = prefix.trim_end_matches('/'),
			static_prefix = self.is_static().then_some("static_").unwrap_or_default(),
			width = self.width,
			height = self.height,
			ext = self.file_extension()
		)
	}
}

#[derive(Debug)]
pub struct Images {
	pub images: Vec<Image>,
}

pub fn process_job(backend: DecoderBackend, job: &Job, data: Cow<'_, [u8]>) -> Result<Images> {
	let mut decoder = backend.build(job, data)?;

	let info = decoder.info();

	let formats = job.task.formats().collect::<HashSet<_>>();
	let mut scales = job.task.scales.iter().cloned().map(|s| s as usize).collect::<Vec<_>>();

	// Sorts the scales from smallest to largest.
	scales.sort();

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

	let (preserve_aspect_height, preserve_aspect_width) = match job.task.resize_method() {
		task::ResizeMethod::Fit => (true, true),
		task::ResizeMethod::Stretch => (false, false),
		task::ResizeMethod::PadBottomLeft => (false, false),
		task::ResizeMethod::PadBottomRight => (false, false),
		task::ResizeMethod::PadTopLeft => (false, false),
		task::ResizeMethod::PadTopRight => (false, false),
		task::ResizeMethod::PadCenter => (false, false),
		task::ResizeMethod::PadCenterLeft => (false, false),
		task::ResizeMethod::PadCenterRight => (false, false),
		task::ResizeMethod::PadTopCenter => (false, false),
		task::ResizeMethod::PadBottomCenter => (false, false),
		task::ResizeMethod::PadTop => (false, true),
		task::ResizeMethod::PadBottom => (false, true),
		task::ResizeMethod::PadLeft => (true, false),
		task::ResizeMethod::PadRight => (true, false),
	};

	let upscale = job.task.upscale().into();

	let scales = ScalingOptions {
		input_height: info.height,
		input_width: info.width,
		input_image_scaling: job.task.input_image_scaling,
		clamp_aspect_ratio: job.task.clamp_aspect_ratio,
		scales,
		aspect_ratio: job
			.task
			.aspect_ratio
			.as_ref()
			.map(|r| Ratio::new(r.numerator as usize, r.denominator as usize))
			.unwrap_or(Ratio::ONE),
		upscale,
		preserve_aspect_height,
		preserve_aspect_width,
	}
	.compute();

	// let base_width = input_width as f64 / job.task.aspect_width as f64;
	let mut resizers = scales
		.into_iter()
		.map(|scale| {
			(
				scale,
				ImageResizer::new(ImageResizerTarget {
					height: scale.height,
					width: scale.width,
					algorithm: job.task.resize_algorithm(),
					method: job.task.resize_method(),
					upscale: upscale.is_yes(),
				}),
				Vec::with_capacity(info.frame_count),
			)
		})
		.collect::<Vec<_>>();

	let mut frame_hashes = HashMap::new();
	let mut frame_order = Vec::with_capacity(info.frame_count);
	let mut count = 0;

	tracing::debug!("decoding frames");

	while let Some(frame) = decoder.decode()? {
		let hash = sha2::Sha256::digest(frame.image.buf().as_bytes());
		if let Some(idx) = frame_hashes.get(&hash) {
			if let Some((last_idx, last_duration)) = frame_order.last_mut() {
				if last_idx == idx {
					*last_duration += frame.duration_ts;
				} else {
					frame_order.push((*idx, frame.duration_ts));
				}
			} else {
				frame_order.push((*idx, frame.duration_ts));
			}
		} else {
			frame_hashes.insert(hash, count);
			frame_order.push((count, frame.duration_ts));

			count += 1;
			for (_, resizer, frames) in resizers.iter_mut() {
				frames.push(resizer.resize(&frame)?);
			}
		}
	}

	tracing::debug!("decoded frames: {count}");

	// We no longer need the decoder so we can free it.
	drop(decoder);

	struct Stack {
		static_encoders: Vec<AnyEncoder>,
		animation_encoders: Vec<AnyEncoder>,
	}

	let mut stacks = resizers
		.iter()
		.map(|(_, _, frames)| {
			Ok(Stack {
				static_encoders: static_formats
					.iter()
					.map(|&frontend| frontend.build(static_settings))
					.collect::<Result<Vec<_>>>()?,
				animation_encoders: if frames.len() > 1 {
					animation_formats
						.iter()
						.map(|&frontend| frontend.build(anim_settings))
						.collect::<Result<Vec<_>>>()?
				} else {
					Vec::new()
				},
			})
		})
		.collect::<Result<Vec<_>>>()?;

	for (stack, frames) in stacks.iter_mut().zip(resizers.iter_mut().map(|(_, _, frames)| frames)) {
		for encoder in stack.animation_encoders.iter_mut() {
			for (idx, timing) in frame_order.iter() {
				let frame = &mut frames[*idx];
				frame.duration_ts = *timing;
				encoder.add_frame(frame)?;
			}

			tracing::debug!("added frames to animation encoder: {count} => {:?}", encoder.info().frontend);
		}

		for encoder in stack.static_encoders.iter_mut() {
			encoder.add_frame(&frames[0])?;
			tracing::debug!("added frame to static encoder: 1 => {:?}", encoder.info().frontend);
		}
	}

	let mut images = Vec::new();

	for stack in stacks.into_iter() {
		for encoder in stack.animation_encoders.into_iter() {
			let info = encoder.info();
			let output = encoder.finish()?;
			images.push(Image {
				width: info.width,
				height: info.height,
				frame_count: info.frame_count,
				duration: info.duration as f64 / info.timescale as f64,
				encoder: info.frontend,
				data: output.into(),
				loop_count: info.loop_count,
				request: match info.frontend {
					EncoderFrontend::Gifski => ImageFormat::Gif,
					EncoderFrontend::LibAvif => ImageFormat::Avif,
					EncoderFrontend::LibWebp => ImageFormat::Webp,
					EncoderFrontend::Png => unreachable!(),
				},
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
				data: output.into(),
				loop_count: info.loop_count,
				request: match info.frontend {
					EncoderFrontend::LibAvif => ImageFormat::AvifStatic,
					EncoderFrontend::LibWebp => ImageFormat::WebpStatic,
					EncoderFrontend::Png => ImageFormat::PngStatic,
					EncoderFrontend::Gifski => unreachable!(),
				},
			});
		}
	}

	Ok(Images { images })
}
