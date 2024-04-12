use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use bytes::Bytes;
use pb::scuffle::platform::internal::types::ImageFormat;
use rgb::ComponentBytes;
use sha2::Digest;

use super::decoder::{Decoder, DecoderBackend, LoopCount};
use super::encoder::{AnyEncoder, Encoder, EncoderFrontend, EncoderSettings};
use super::resize::{ImageResizer, ImageResizerTarget};
use crate::database::Job;
use crate::processor::error::{ProcessorError, Result};

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
			prefix.trim_end_matches('/'),
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

pub fn process_job(backend: DecoderBackend, job: &Job, data: Cow<'_, [u8]>) -> Result<Images> {
	let mut decoder = backend.build(job, data)?;

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
			(job.task.base_width as f64, job.task.base_height as f64)
		} else {
			(width, height)
		}
	};

	let mut resizers = scales
		.iter()
		.map(|scale| {
			(
				*scale,
				ImageResizer::new(ImageResizerTarget {
					height: base_height.ceil() as usize * scale,
					width: base_width.ceil() as usize * scale,
					algorithm: job.task.resize_algorithm(),
					method: job.task.resize_method(),
					upscale: job.task.upscale,
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
		scale: usize,
		static_encoders: Vec<AnyEncoder>,
		animation_encoders: Vec<AnyEncoder>,
	}

	let mut stacks = resizers
		.iter_mut()
		.map(|(scale, _, frames)| {
			Ok(Stack {
				scale: *scale,
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
				data: output.into(),
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
