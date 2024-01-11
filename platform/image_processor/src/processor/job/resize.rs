use anyhow::Context;
use imgref::Img;
use pb::scuffle::platform::internal::image_processor::task::{ResizeAlgorithm, ResizeMethod};
use rgb::ComponentBytes;

use super::frame::Frame;
use crate::processor::error::{ProcessorError, Result};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ImageResizerTarget {
	pub width: usize,
	pub height: usize,
	pub algorithm: ResizeAlgorithm,
	pub method: ResizeMethod,
}

pub const fn algo_to_opencv(algo: ResizeAlgorithm) -> i32 {
	match algo {
		ResizeAlgorithm::Nearest => opencv::imgproc::INTER_NEAREST,
		ResizeAlgorithm::Linear => opencv::imgproc::INTER_LINEAR,
		ResizeAlgorithm::Cubic => opencv::imgproc::INTER_CUBIC,
		ResizeAlgorithm::Area => opencv::imgproc::INTER_AREA,
		ResizeAlgorithm::Lanczos4 => opencv::imgproc::INTER_LANCZOS4,
	}
}

/// Resizes images to the given target size.
pub struct ImageResizer {
	target: ImageResizerTarget,
	buffer: Vec<rgb::RGBA8>,
	padding_buffer: Vec<rgb::RGBA8>,
}

impl std::fmt::Debug for ImageResizer {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ImageResizer").field("target", &self.target).finish()
	}
}

/// Safety: this function is unsafe because Mat must not outlive self.
/// The caller must ensure that the returned Mat is dropped before self is
/// dropped.
unsafe fn make_mat(buffer: &[rgb::RGBA8], width: usize, height: usize) -> Result<opencv::core::Mat> {
	let data = buffer.as_bytes().as_ptr() as *mut std::ffi::c_void;
	opencv::core::Mat::new_rows_cols_with_data(
		height as i32,
		width as i32,
		opencv::core::CV_8UC4,
		data,
		opencv::core::Mat_AUTO_STEP,
	)
	.context("opencv mat new rows cols with data")
	.map_err(ProcessorError::ImageResize)
}

impl ImageResizer {
	pub fn new(target: ImageResizerTarget) -> Self {
		Self {
			target,
			buffer: vec![rgb::RGBA8::default(); target.width * target.height],
			padding_buffer: vec![rgb::RGBA8::default(); target.width * target.height],
		}
	}

	/// Resize the given frame to the target size, returning a reference to the
	/// resized frame. After this function returns original frame can be
	/// dropped, the returned frame is valid for the lifetime of the Resizer.
	pub fn resize(&mut self, frame: &Frame) -> Result<Frame> {
		let _abort_guard = common::task::AbortGuard::new();

		// Safety: `data` is a valid pointer, and even tho we dont own it, we do not
		// make mat mutable so we wont modify it.        `data` is valid for the
		// lifetime of `frame`, which is the lifetime of the function.
		let mat = unsafe { make_mat(frame.image.buf(), frame.image.width(), frame.image.height()) }?;

		// Safety: We drop the returned Mat before self is dropped.
		let (width, height) = if self.target.method == ResizeMethod::Exact {
			(self.target.width, self.target.height)
		} else {
			let (width, height) = if frame.image.width() > frame.image.height() {
				let width = self.target.width as f64;
				let height = frame.image.height() as f64 / frame.image.width() as f64 * width;
				(width, height)
			} else {
				let height = self.target.height as f64;
				let width = frame.image.width() as f64 / frame.image.height() as f64 * height;
				(width, height)
			};

			let (width, height) = (width.round() as usize, height.round() as usize);

			(width, height)
		};

		let mut buffer_mat = unsafe { make_mat(&self.buffer, width, height) }?;

		opencv::imgproc::resize(
			&mat,
			&mut buffer_mat,
			opencv::core::Size {
				width: width as _,
				height: height as _,
			},
			0.0,
			0.0,
			algo_to_opencv(self.target.algorithm),
		)
		.context("opencv imgproc resize")
		.map_err(ProcessorError::ImageResize)?;

		let image =
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

				let width = width + left + right;
				let height = height + top + bottom;

				// OpenCV allows for in-place operations, but the original Mat has been sized
				// with a capacity. We need to create a new Mat with the correct size.
				let mut padded_mat = unsafe { make_mat(&self.padding_buffer, width, height) }?;

				opencv::core::copy_make_border(
					&buffer_mat,
					&mut padded_mat,
					top as i32,
					bottom as i32,
					left as i32,
					right as i32,
					opencv::core::BorderTypes::BORDER_CONSTANT as _,
					opencv::core::Scalar::all(0.0),
				)
				.context("opencv imgproc copy make border")
				.map_err(ProcessorError::ImageResize)?;

				Img::new(self.padding_buffer.clone(), width, height)
			} else {
				Img::new(self.buffer.clone(), width, height)
			};

		Ok(Frame {
			image,
			duration_ts: frame.duration_ts,
		})
	}
}
