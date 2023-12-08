use std::borrow::Cow;
use std::path::Path;
use std::ptr::NonNull;

use anyhow::{anyhow, Context};

use super::{Decoder, DecoderBackend, DecoderInfo, FrameRef, LoopCount};
use crate::database::Job;
use crate::processor::error::{ProcessorError, Result};
use crate::processor::job::frame::FrameCow;
use crate::processor::job::libavif::{AvifError, AvifRgbImage};
use crate::processor::job::smart_object::SmartPtr;

#[derive(Debug)]
pub struct AvifDecoder<'data> {
	decoder: SmartPtr<libavif_sys::avifDecoder>,
	info: DecoderInfo,
	_data: Cow<'data, [u8]>,
	img: AvifRgbImage,
	total_duration: u64,
	max_input_duration: u64,
}

impl AvifDecoder<'_> {
	pub fn new(job: &Job, input_path: &Path) -> Result<Self> {
		let data = std::fs::read(input_path).map_err(ProcessorError::FileRead)?;

		let mut decoder = SmartPtr::new(
			NonNull::new(unsafe { libavif_sys::avifDecoderCreate() })
				.ok_or(AvifError::OutOfMemory)
				.context("failed to create avif decoder")
				.map_err(ProcessorError::AvifDecode)?,
			|ptr| {
				// Safety: The decoder is valid.
				unsafe {
					libavif_sys::avifDecoderDestroy(ptr.as_ptr());
				}
			},
		);

		let max_input_width = job.task.limits.as_ref().map(|l| l.max_input_width).unwrap_or(0);
		let max_input_height = job.task.limits.as_ref().map(|l| l.max_input_height).unwrap_or(0);
		if max_input_height != 0 && max_input_width != 0 {
			decoder.as_mut().imageDimensionLimit = max_input_width * max_input_height;
		}

		let max_input_frame_count = job.task.limits.as_ref().map(|l| l.max_input_frame_count).unwrap_or(0);
		if max_input_frame_count != 0 {
			decoder.as_mut().imageCountLimit = max_input_frame_count;
		}

		// Safety: The decoder is valid.
		let io = NonNull::new(unsafe { libavif_sys::avifIOCreateMemoryReader(data.as_ptr(), data.len()) })
			.ok_or(AvifError::OutOfMemory)
			.context("failed to create avif io")
			.map_err(ProcessorError::AvifDecode)?;

		// Set the io pointer.
		decoder.as_mut().io = io.as_ptr();

		// Parse the data.
		AvifError::from_code(unsafe { libavif_sys::avifDecoderParse(decoder.as_ptr()) })
			.context("failed to parse avif")
			.map_err(ProcessorError::AvifDecode)?;

		let image = AvifRgbImage::new(decoder.as_ref());

		let info = DecoderInfo {
			width: image.width as usize,
			height: image.height as usize,
			loop_count: if decoder.as_ref().repetitionCount <= 0 {
				LoopCount::Infinite
			} else {
				LoopCount::Finite(decoder.as_ref().repetitionCount as usize)
			},
			frame_count: decoder.as_ref().imageCount.max(0) as _,
			timescale: decoder.as_ref().timescale,
		};

		let max_input_duration_ms = job.task.limits.as_ref().map(|l| l.max_input_duration_ms).unwrap_or(0);

		if max_input_width != 0 && info.width > max_input_width as usize {
			return Err(ProcessorError::AvifDecode(anyhow!("input width exceeds limit")));
		}

		if max_input_height != 0 && info.height > max_input_height as usize {
			return Err(ProcessorError::AvifDecode(anyhow!("input height exceeds limit")));
		}

		if max_input_frame_count != 0 && info.frame_count > max_input_frame_count as usize {
			return Err(ProcessorError::AvifDecode(anyhow!("input frame count exceeds limit")));
		}

		Ok(Self {
			_data: Cow::Owned(data),
			img: AvifRgbImage::new(decoder.as_ref()),
			decoder,
			max_input_duration: max_input_duration_ms as u64 * info.timescale / 1000,
			total_duration: 0,
			info,
		})
	}
}

impl Decoder for AvifDecoder<'_> {
	fn backend(&self) -> DecoderBackend {
		DecoderBackend::LibAvif
	}

	fn info(&self) -> DecoderInfo {
		self.info
	}

	fn decode(&mut self) -> Result<Option<FrameCow<'_>>> {
		if AvifError::from_code(unsafe { libavif_sys::avifDecoderNextImage(self.decoder.as_ptr()) }).is_err() {
			return Ok(None);
		}

		AvifError::from_code(unsafe { libavif_sys::avifImageYUVToRGB(self.decoder.as_ref().image, &mut *self.img) })
			.context("failed to convert YUV to RGB")
			.map_err(ProcessorError::AvifDecode)?;

		let duration_ts = self.decoder.as_ref().imageTiming.durationInTimescales;
		self.total_duration += duration_ts;

		if self.max_input_duration != 0 && self.total_duration > self.max_input_duration {
			return Err(ProcessorError::AvifDecode(anyhow!("input duration exceeds limit")));
		}

		Ok(Some(
			FrameRef {
				image: self.img.data(),
				duration_ts,
			}
			.into(),
		))
	}
}
