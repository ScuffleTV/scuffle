use std::ptr::NonNull;

use anyhow::Context;
use libwebp_sys::WebPMuxAnimParams;

use super::{Encoder, EncoderFrontend, EncoderInfo, EncoderSettings};
use crate::processor::error::{ProcessorError, Result};
use crate::processor::job::decoder::LoopCount;
use crate::processor::job::frame::Frame;
use crate::processor::job::libwebp::{zero_memory_default, WebPError};
use crate::processor::job::smart_object::{SmartObject, SmartPtr};

pub struct WebpEncoder {
	config: libwebp_sys::WebPConfig,
	settings: EncoderSettings,
	picture: SmartObject<libwebp_sys::WebPPicture>,
	encoder: Option<SmartPtr<libwebp_sys::WebPAnimEncoder>>,
	first_duration: Option<u64>,
	info: EncoderInfo,
	static_image: bool,
}

fn wrap_error(status: i32, err: &'static str, message: &'static str) -> Result<()> {
	if status == 0 {
		Err(WebPError::UnknownError(err))
			.context(message)
			.map_err(ProcessorError::WebPEncode)
	} else {
		Ok(())
	}
}

impl WebpEncoder {
	pub fn new(settings: EncoderSettings) -> Result<Self> {
		let mut config = zero_memory_default::<libwebp_sys::WebPConfig>();

		config.thread_level = 1;

		wrap_error(
			unsafe { libwebp_sys::WebPConfigInit(&mut config) },
			"failed to initialize webp config",
			"libwebp_sys::WebPConfigInit",
		)?;

		let mut picture = SmartObject::new(zero_memory_default::<libwebp_sys::WebPPicture>(), |ptr| unsafe {
			libwebp_sys::WebPPictureFree(ptr);
		});

		wrap_error(
			unsafe { libwebp_sys::WebPPictureInit(&mut *picture) },
			"failed to initialize webp picture",
			"libwebp_sys::WebPPictureInit",
		)?;

		picture.use_argb = 1;

		Ok(Self {
			config,
			settings,
			picture,
			encoder: None,
			first_duration: None,
			static_image: settings.static_image,
			info: EncoderInfo {
				duration: 0,
				frame_count: 0,
				frontend: EncoderFrontend::LibWebp,
				height: 0,
				loop_count: settings.loop_count,
				timescale: settings.timescale,
				width: 0,
			},
		})
	}

	fn timestamp(&self) -> u64 {
		self.info.duration * 1000 / self.settings.timescale
	}

	fn flush_frame(&mut self, duration: u64) -> Result<()> {
		let _abort_guard = utils::task::AbortGuard::new();

		// Safety: The picture is valid.
		wrap_error(
			unsafe {
				libwebp_sys::WebPAnimEncoderAdd(
					self.encoder.as_mut().unwrap().as_ptr(),
					&mut *self.picture,
					self.timestamp() as _,
					&self.config,
				)
			},
			"failed to add webp frame",
			"libwebp_sys::WebPAnimEncoderAdd",
		)?;

		self.info.duration += duration;

		Ok(())
	}
}

impl Encoder for WebpEncoder {
	fn info(&self) -> EncoderInfo {
		self.info
	}

	fn add_frame(&mut self, frame: &Frame) -> Result<()> {
		let _abort_guard = utils::task::AbortGuard::new();

		if self.first_duration.is_none() && self.encoder.is_none() {
			self.picture.width = frame.image.width() as _;
			self.picture.height = frame.image.height() as _;
			self.first_duration = Some(frame.duration_ts);
		} else if let Some(first_duration) = self.first_duration.take() {
			if self.static_image {
				return Err(ProcessorError::WebPEncode(anyhow::anyhow!("static image already added")));
			}

			let encoder = SmartPtr::new(
				NonNull::new(unsafe {
					libwebp_sys::WebPAnimEncoderNew(
						self.picture.width,
						self.picture.height,
						&libwebp_sys::WebPAnimEncoderOptions {
							allow_mixed: 1,
							anim_params: WebPMuxAnimParams {
								bgcolor: 0,
								loop_count: match self.settings.loop_count {
									LoopCount::Infinite => 0,
									LoopCount::Finite(count) => count as _,
								},
							},
							kmax: 0,
							kmin: 0,
							verbose: 0,
							minimize_size: 0,
							padding: [0; 4],
						},
					)
				})
				.ok_or(WebPError::OutOfMemory)
				.context("failed to create webp encoder")
				.map_err(ProcessorError::WebPEncode)?,
				|encoder| {
					// Safety: The encoder is valid.
					unsafe {
						libwebp_sys::WebPAnimEncoderDelete(encoder.as_ptr());
					}
				},
			);

			self.encoder = Some(encoder);
			self.flush_frame(first_duration)?;
		}

		wrap_error(
			unsafe {
				libwebp_sys::WebPPictureImportRGBA(
					&mut *self.picture,
					frame.image.buf().as_ptr() as _,
					frame.image.width() as i32 * 4,
				)
			},
			"failed to import webp frame",
			"libwebp_sys::WebPPictureImportRGBA",
		)?;

		if self.encoder.is_some() {
			self.flush_frame(frame.duration_ts)?;
		}

		self.info.frame_count += 1;
		self.info.width = frame.image.width() as _;
		self.info.height = frame.image.height() as _;

		Ok(())
	}

	fn finish(mut self) -> Result<Vec<u8>> {
		let _abort_guard = utils::task::AbortGuard::new();

		let timestamp = self.timestamp();

		if self.encoder.is_none() && self.first_duration.is_none() {
			Err(ProcessorError::WebPEncode(anyhow::anyhow!("no frames added")))
		} else if let Some(mut encoder) = self.encoder {
			wrap_error(
				unsafe {
					libwebp_sys::WebPAnimEncoderAdd(encoder.as_mut(), std::ptr::null_mut(), timestamp as _, std::ptr::null())
				},
				"failed to add null webp frame",
				"libwebp_sys::WebPAnimEncoderAdd",
			)?;

			let mut webp_data = SmartObject::new(zero_memory_default::<libwebp_sys::WebPData>(), |ptr| unsafe {
				libwebp_sys::WebPDataClear(ptr);
			});

			// Safety: The data is valid.
			unsafe { libwebp_sys::WebPDataInit(&mut *webp_data) };

			wrap_error(
				unsafe { libwebp_sys::WebPAnimEncoderAssemble(encoder.as_mut(), &mut *webp_data) },
				"failed to assemble webp",
				"libwebp_sys::WebPAnimEncoderAssemble",
			)?;

			let webp_data = webp_data.free();

			let mut data = NonNull::new(webp_data.bytes as _)
				.ok_or(WebPError::OutOfMemory)
				.context("failed to get output data")
				.map_err(ProcessorError::WebPEncode)?;

			// Safety: The data is valid and we are taking ownership of it.
			let vec = unsafe { std::vec::Vec::from_raw_parts(data.as_mut(), webp_data.size, webp_data.size) };

			Ok(vec)
		} else {
			let mut memory_writer = SmartObject::new(zero_memory_default::<libwebp_sys::WebPMemoryWriter>(), |ptr| unsafe {
				libwebp_sys::WebPMemoryWriterClear(ptr);
			});

			// Safety: The functions are correct, but the library requires picture.writer to
			// be a "safe" function and we only have a "unsafe" function.
			self.picture.writer = Some(unsafe { std::mem::transmute(libwebp_sys::WebPMemoryWrite as *const ()) });
			self.picture.custom_ptr = &mut *memory_writer as *mut _ as _;

			// Safety: The picture is valid.
			wrap_error(
				unsafe { libwebp_sys::WebPEncode(&self.config, &mut *self.picture) },
				"failed to encode webp",
				"libwebp_sys::WebPEncode",
			)?;

			let memory_writer = memory_writer.free();

			let mut data = NonNull::new(memory_writer.mem)
				.ok_or(WebPError::OutOfMemory)
				.context("failed to get output data")
				.map_err(ProcessorError::WebPEncode)?;

			// Safety: The data is valid and we are taking ownership of it.
			let vec = unsafe { std::vec::Vec::from_raw_parts(data.as_mut(), memory_writer.size, memory_writer.max_size) };

			Ok(vec)
		}
	}
}
