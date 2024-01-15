use std::borrow::Cow;
use std::ptr::NonNull;

use anyhow::{anyhow, Context};
use imgref::Img;

use super::{Decoder, DecoderBackend, DecoderInfo, LoopCount};
use crate::database::Job;
use crate::processor::error::{DecoderError, ProcessorError, Result};
use crate::processor::job::frame::Frame;
use crate::processor::job::libwebp::{zero_memory_default, WebPError};
use crate::processor::job::smart_object::SmartPtr;

pub struct WebpDecoder<'data> {
	info: DecoderInfo,
	decoder: SmartPtr<libwebp_sys::WebPAnimDecoder>,
	_data: Cow<'data, [u8]>,
	timestamp: i32,
	total_duration: u64,
	max_input_duration: u64,
}

impl<'data> WebpDecoder<'data> {
	pub fn new(job: &Job, data: Cow<'data, [u8]>) -> Result<Self> {
		let max_input_width = job.task.limits.as_ref().map(|l| l.max_input_width).unwrap_or(0);
		let max_input_height = job.task.limits.as_ref().map(|l| l.max_input_height).unwrap_or(0);
		let max_input_frame_count = job.task.limits.as_ref().map(|l| l.max_input_frame_count).unwrap_or(0);
		let max_input_duration_ms = job.task.limits.as_ref().map(|l| l.max_input_duration_ms).unwrap_or(0);

		let decoder = SmartPtr::new(
			NonNull::new(unsafe {
				libwebp_sys::WebPAnimDecoderNew(
					&libwebp_sys::WebPData {
						bytes: data.as_ptr(),
						size: data.len(),
					},
					std::ptr::null(),
				)
			})
			.ok_or(WebPError::OutOfMemory)
			.context("failed to create webp decoder")
			.map_err(DecoderError::Other)
			.map_err(ProcessorError::WebPDecode)?,
			|decoder| {
				// Safety: The decoder is valid.
				unsafe {
					libwebp_sys::WebPAnimDecoderDelete(decoder.as_ptr());
				}
			},
		);

		let mut info = zero_memory_default::<libwebp_sys::WebPAnimInfo>();

		// Safety: both pointers are valid and the decoder is valid.
		if unsafe { libwebp_sys::WebPAnimDecoderGetInfo(decoder.as_ptr(), &mut info) } == 0 {
			return Err(ProcessorError::WebPDecode(DecoderError::Other(anyhow!(
				"failed to get webp info"
			))));
		}

		if max_input_width != 0 && info.canvas_width > max_input_width {
			return Err(ProcessorError::WebPDecode(DecoderError::TooWide(info.canvas_width as i32)));
		}

		if max_input_height != 0 && info.canvas_height > max_input_height {
			return Err(ProcessorError::WebPDecode(DecoderError::TooHigh(info.canvas_height as i32)));
		}

		if max_input_frame_count != 0 && info.frame_count > max_input_frame_count {
			return Err(ProcessorError::WebPDecode(DecoderError::TooManyFrames(
				info.frame_count as i64,
			)));
		}

		Ok(Self {
			info: DecoderInfo {
				width: info.canvas_width as _,
				height: info.canvas_height as _,
				loop_count: match info.loop_count {
					0 => LoopCount::Infinite,
					_ => LoopCount::Finite(info.loop_count as _),
				},
				frame_count: info.frame_count as _,
				timescale: 1000,
			},
			max_input_duration: max_input_duration_ms as u64,
			decoder,
			_data: data,
			total_duration: 0,
			timestamp: 0,
		})
	}
}

impl Decoder for WebpDecoder<'_> {
	fn backend(&self) -> DecoderBackend {
		DecoderBackend::LibWebp
	}

	fn info(&self) -> DecoderInfo {
		self.info
	}

	fn decode(&mut self) -> Result<Option<Frame>> {
		let _abort_guard = common::task::AbortGuard::new();

		let mut buf = std::ptr::null_mut();
		let previous_timestamp = self.timestamp;

		// Safety: The buffer is a valid pointer to a null ptr, timestamp is a valid
		// pointer to i32, and the decoder is valid.
		let result = unsafe { libwebp_sys::WebPAnimDecoderGetNext(self.decoder.as_ptr(), &mut buf, &mut self.timestamp) };

		// If 0 is returned, the animation is over.
		if result == 0 {
			return Ok(None);
		}

		let buf = NonNull::new(buf)
			.ok_or(WebPError::OutOfMemory)
			.context("failed to get webp frame")
			.map_err(DecoderError::Other)
			.map_err(ProcessorError::WebPDecode)?;

		let image =
			unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const rgb::RGBA8, self.info.width * self.info.height) };

		let duration_ts = (self.timestamp - previous_timestamp).max(0) as u64;
		self.total_duration += duration_ts;

		if self.max_input_duration != 0 && self.total_duration > self.max_input_duration {
			return Err(ProcessorError::WebPDecode(DecoderError::TooLong(self.total_duration as i64)));
		}

		Ok(Some(Frame {
			image: Img::new(image.to_vec(), self.info.width, self.info.height),
			duration_ts: (self.timestamp - previous_timestamp).max(0) as _,
		}))
	}
}
