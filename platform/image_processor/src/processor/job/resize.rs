use anyhow::Context;
use fast_image_resize as fr;
use imgref::Img;
use pb::scuffle::platform::internal::image_processor::task::{ResizeAlgorithm, ResizeMethod};
use rgb::{ComponentBytes, RGBA};

use super::frame::Frame;
use crate::processor::error::{ProcessorError, Result};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ImageResizerTarget {
	pub width: usize,
	pub height: usize,
	pub algorithm: ResizeAlgorithm,
	pub method: ResizeMethod,
	pub upscale: bool,
}

/// Resizes images to the given target size.
pub struct ImageResizer {
	resizer: fr::Resizer,
	target: ImageResizerTarget,
}

impl std::fmt::Debug for ImageResizer {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ImageResizer").field("target", &self.target).finish()
	}
}

impl ImageResizer {
	pub fn new(target: ImageResizerTarget) -> Self {
		Self {
			resizer: fr::Resizer::new(match target.algorithm {
				ResizeAlgorithm::Nearest => fr::ResizeAlg::Nearest,
				ResizeAlgorithm::Box => fr::ResizeAlg::Convolution(fr::FilterType::Box),
				ResizeAlgorithm::Bilinear => fr::ResizeAlg::Convolution(fr::FilterType::Bilinear),
				ResizeAlgorithm::Hamming => fr::ResizeAlg::Convolution(fr::FilterType::Hamming),
				ResizeAlgorithm::CatmullRom => fr::ResizeAlg::Convolution(fr::FilterType::CatmullRom),
				ResizeAlgorithm::Mitchell => fr::ResizeAlg::Convolution(fr::FilterType::Mitchell),
				ResizeAlgorithm::Lanczos3 => fr::ResizeAlg::Convolution(fr::FilterType::Lanczos3),
			}),
			target,
		}
	}

	/// Resize the given frame to the target size, returning a reference to the
	/// resized frame. After this function returns original frame can be
	/// dropped, the returned frame is valid for the lifetime of the Resizer.
	pub fn resize(&mut self, frame: &Frame) -> Result<Frame> {
		let _abort_guard = common::task::AbortGuard::new();

		let (width, height) = if self.target.method == ResizeMethod::Exact {
			(self.target.width, self.target.height)
		} else {
			let (mut width, mut height) = if frame.image.width() > frame.image.height() {
				let width = self.target.width as f64;
				let height = frame.image.height() as f64 / frame.image.width() as f64 * width;
				(width, height)
			} else {
				let height = self.target.height as f64;
				let width = frame.image.width() as f64 / frame.image.height() as f64 * height;
				(width, height)
			};

			if width > self.target.width as f64 {
				height = height / width * self.target.width as f64;
				width = self.target.width as f64;
			} else if height > self.target.height as f64 {
				width = width / height * self.target.height as f64;
				height = self.target.height as f64;
			}

			let (width, height) = (width.round() as usize, height.round() as usize);

			(width, height)
		};

		let (mut dst_image, crop_box) =
			if self.target.method != ResizeMethod::Fit && (width != self.target.width || height != self.target.height) {
				let height_delta = self.target.height - height;
				let width_delta = self.target.width - width;

				let (top, bottom, left, right) = match self.target.method {
					ResizeMethod::PadBottomLeft => (0, height_delta, width_delta, 0),
					ResizeMethod::PadBottomRight => (0, height_delta, 0, width_delta),
					ResizeMethod::PadTopLeft => (height_delta, 0, width_delta, 0),
					ResizeMethod::PadTopRight => (height_delta, 0, 0, width_delta),
					ResizeMethod::PadCenter => {
						let top = height_delta / 2;
						let bottom = height_delta - top;
						let left = width_delta / 2;
						let right = width_delta - left;
						(top, bottom, left, right)
					}
					ResizeMethod::PadCenterLeft => {
						let top = height_delta / 2;
						let bottom = height_delta - top;
						(top, bottom, width_delta, 0)
					}
					ResizeMethod::PadCenterRight => {
						let top = height_delta / 2;
						let bottom = height_delta - top;
						(top, bottom, 0, width_delta)
					}
					ResizeMethod::PadTopCenter => {
						let left = width_delta / 2;
						let right = width_delta - left;
						(height_delta, 0, left, right)
					}
					ResizeMethod::PadBottomCenter => {
						let left = width_delta / 2;
						let right = width_delta - left;
						(0, height_delta, left, right)
					}
					ResizeMethod::PadTop => (height_delta, 0, 0, 0),
					ResizeMethod::PadBottom => (0, height_delta, 0, 0),
					ResizeMethod::PadLeft => (0, 0, width_delta, 0),
					ResizeMethod::PadRight => (0, 0, 0, width_delta),
					ResizeMethod::Exact => unreachable!(),
					ResizeMethod::Fit => unreachable!(),
				};

				let total_width = width + left + right;
				let total_height = height + top + bottom;

				let dst_image = fr::Image::new(
					(total_width as u32).try_into().unwrap(),
					(total_height as u32).try_into().unwrap(),
					fr::pixels::PixelType::U8x4,
				);
				(
					dst_image,
					fr::CropBox {
						height: (height as u32).try_into().unwrap(),
						width: (width as u32).try_into().unwrap(),
						left: left as u32,
						top: top as u32,
					},
				)
			} else {
				let dst_image = fr::Image::new(
					(width as u32).try_into().unwrap(),
					(height as u32).try_into().unwrap(),
					fr::pixels::PixelType::U8x4,
				);
				(
					dst_image,
					fr::CropBox {
						height: (height as u32).try_into().unwrap(),
						width: (width as u32).try_into().unwrap(),
						left: 0,
						top: 0,
					},
				)
			};

		let mut cropped_dst_view = dst_image.view_mut().crop(crop_box).unwrap();

		let size = frame.image.buf().len();

		let src = fr::Image::from_slice_u8(
			(frame.image.width() as u32).try_into().unwrap(),
			(frame.image.height() as u32).try_into().unwrap(),
			unsafe { std::slice::from_raw_parts_mut(frame.image.buf().as_ptr() as *mut u8, size * 4) },
			fr::pixels::PixelType::U8x4,
		)
		.unwrap();
		self.resizer
			.resize(&src.view(), &mut cropped_dst_view)
			.context("failed to resize image")
			.map_err(ProcessorError::ImageResize)?;
		drop(src);

		let width = dst_image.width().get() as usize;
		let height = dst_image.height().get() as usize;
		let buffer = dst_image.into_vec();

		let buffer = unsafe {
			let buf = buffer.into_boxed_slice();
			let size = buf.len();
			let ptr = Box::into_raw(buf) as *mut u8;

			let new_size = size / 4;
			assert!(new_size * 4 == size, "image buffer size mismatch");

			Vec::from_raw_parts(ptr as *mut RGBA<u8>, new_size, new_size)
		};

		assert_eq!(buffer.as_bytes().len(), width * height * 4, "image buffer size mismatch");

		Ok(Frame {
			image: Img::new(buffer, width, height),
			duration_ts: frame.duration_ts,
		})
	}
}
