use std::num::NonZeroU32;

use fast_image_resize as fr;
use fr::CropBox;
use rgb::ComponentBytes;
use scuffle_image_processor_proto::{output, scaling, Output, ResizeAlgorithm, ResizeMethod};

use super::decoder::DecoderInfo;
use super::frame::{Frame, FrameRef};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Dimensions {
	pub width: usize,
	pub height: usize,
}

impl Dimensions {
	fn new(width: usize, height: usize) -> Self {
		Self { width, height }
	}

	fn aspect_ratio(&self) -> f64 {
		self.width as f64 / self.height as f64
	}

	fn convert_aspect_ratio(&self, aspect_ratio: f64) -> Self {
		if aspect_ratio > self.aspect_ratio() {
			Self::new(self.width, (self.width as f64 / aspect_ratio) as usize)
		} else {
			Self::new((self.height as f64 * aspect_ratio) as usize, self.height)
		}
	}
}

enum ImageRef<'a> {
	Ref((&'a fr::Image<'a>, Option<fr::CropBox>)),
	Owned((fr::Image<'a>, Option<fr::CropBox>)),
}

impl ImageRef<'_> {
	fn crop(&self) -> Option<fr::CropBox> {
		match self {
			ImageRef::Owned((_, c)) => *c,
			ImageRef::Ref((_, c)) => *c,
		}
	}
}

impl<'a> std::ops::Deref for ImageRef<'a> {
	type Target = fr::Image<'a>;

	fn deref(&self) -> &Self::Target {
		match self {
			ImageRef::Owned(o) => &o.0,
			ImageRef::Ref(r) => r.0,
		}
	}
}

/// Resizes images to the given target size.
pub struct ImageResizer {
	resizer: fr::Resizer,
	input_dims: Dimensions,
	cropped_dims: Dimensions,
	crop: Option<CropBox>,
	resize_dims: Vec<Dimensions>,
	outputs: Vec<ResizeOutputTarget>,
	resize_method: ResizeMethod,
	output_frames: Vec<Frame>,
	disable_resize_chaining: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct ResizeOutputTarget {
	pub dimensions: Dimensions,
	pub index: usize,
	pub scale: Option<u32>,
}

#[derive(thiserror::Error, Debug)]
pub enum ResizeError {
	#[error("crop: {0}")]
	Crop(#[from] fr::CropBoxError),
	#[error("different pixels: {0}")]
	DifferentPixels(#[from] fr::DifferentTypesOfPixelsError),
	#[error("buffer: {0}")]
	Buffer(#[from] fr::ImageBufferError),
	#[error("crop dimensions are larger than the input dimensions")]
	CropDimensions,
	#[error("aspect ratio is too small")]
	AspectTooSmall,
	#[error("aspect ratio is too large")]
	AspectTooLarge,
	#[error("invalid crop")]
	InvalidCrop,
	#[error("missing resize")]
	MissingResize,
	#[error("no valid resize targets")]
	NoValidResizeTargets,
	#[error("impossible resize[{0}] {1}x{2} is larger than the input size ({3}x{4})")]
	ImpossibleResize(usize, usize, usize, usize, usize),
	#[error("input frame has mismatched dimensions")]
	MismatchedDimensions,
	#[error("{0}")]
	Internal(&'static str),
}

impl ImageResizer {
	pub fn new(info: &DecoderInfo, output: &Output) -> Result<Self, ResizeError> {
		let input_dims = Dimensions::new(info.width, info.height);

		// If there is a crop, we should use that aspect ratio instead.
		let cropped_dims = if let Some(crop) = output.crop.as_ref() {
			if crop.width == 0 || crop.height == 0 {
				return Err(ResizeError::InvalidCrop);
			}

			if crop.width + crop.x > info.width as u32 || crop.height + crop.y > info.height as u32 {
				return Err(ResizeError::CropDimensions);
			}

			Dimensions::new(crop.width as usize, crop.height as usize)
		} else {
			input_dims
		};

		let resize_method = output.resize_method();
		let mut target_aspect_ratio = cropped_dims.aspect_ratio();

		if output
			.min_aspect_ratio
			.is_some_and(|min_aspect_ratio| target_aspect_ratio < min_aspect_ratio)
		{
			// If the resize method is one of these, we can't make the aspect ratio larger.
			// Because we are not allowed to pad the left or right.
			if matches!(
				resize_method,
				ResizeMethod::Fit | ResizeMethod::PadTop | ResizeMethod::PadBottom
			) {
				return Err(ResizeError::AspectTooSmall);
			}

			target_aspect_ratio = output.min_aspect_ratio();
		} else if output
			.max_aspect_ratio
			.is_some_and(|max_aspect_ratio| target_aspect_ratio > max_aspect_ratio)
		{
			// If the resize method is one of these, we can't make the aspect ratio smaller.
			// Because we are not allowed to pad the top or bottom.
			if matches!(
				resize_method,
				ResizeMethod::Fit | ResizeMethod::PadLeft | ResizeMethod::PadRight
			) {
				return Err(ResizeError::AspectTooLarge);
			}

			target_aspect_ratio = output.max_aspect_ratio();
		}

		let mut output_targets: Vec<_> = match output.resize.as_ref().ok_or(ResizeError::MissingResize)? {
			output::Resize::Width(widths) => widths
				.values
				.iter()
				.copied()
				.enumerate()
				.map(|(index, width)| ResizeOutputTarget {
					dimensions: Dimensions::new(width as usize, (width as f64 / target_aspect_ratio) as usize),
					index,
					scale: None,
				})
				.collect(),
			output::Resize::Height(heights) => heights
				.values
				.iter()
				.copied()
				.enumerate()
				.map(|(index, height)| ResizeOutputTarget {
					dimensions: Dimensions::new((height as f64 * target_aspect_ratio) as usize, height as usize),
					index,
					scale: None,
				})
				.collect(),
			output::Resize::Scaling(scaling) => {
				let (base_width, base_height) = match scaling.base.clone().ok_or(ResizeError::MissingResize)? {
					scaling::Base::Fixed(scale) => {
						let input = cropped_dims.convert_aspect_ratio(target_aspect_ratio);

						(input.width / scale as usize, input.height / scale as usize)
					}
					scaling::Base::BaseWidth(width) => (width as usize, (width as f64 / target_aspect_ratio) as usize),
					scaling::Base::BaseHeight(height) => ((height as f64 * target_aspect_ratio) as usize, height as usize),
				};

				scaling
					.scales
					.iter()
					.copied()
					.enumerate()
					.map(|(index, scale)| ResizeOutputTarget {
						dimensions: Dimensions::new(base_width * scale as usize, base_height * scale as usize),
						index,
						scale: Some(scale),
					})
					.collect()
			}
		};

		if !output.upscale {
			let input_after_transforms = cropped_dims.convert_aspect_ratio(target_aspect_ratio);

			if output.skip_impossible_resizes {
				output_targets.retain(|target| target.dimensions <= input_after_transforms);
			} else if let Some(target) = output_targets
				.iter()
				.find(|target| target.dimensions > input_after_transforms)
			{
				return Err(ResizeError::ImpossibleResize(
					target.index,
					target.dimensions.width,
					target.dimensions.height,
					input_after_transforms.width,
					input_after_transforms.height,
				));
			}
		}

		// Build the output frames.
		// This is going to be the in the target aspect ratio.
		// therefore needs to be done before we convert the aspect ratio back.
		let output_frames = output_targets
			.iter()
			.map(|target| Frame::new(target.dimensions.width, target.dimensions.height))
			.collect();

		// Convert the apect ratios back to the original aspect ratio.
		// This is because padding is added AFTER we resize the image.
		// Thus we need to resize the image to the target aspect ratio.
		// However if we are stretching the image, we don't need to do this,
		// because we want to warp the image.
		let resize_targets: Vec<_> =
			if target_aspect_ratio != cropped_dims.aspect_ratio() && output.resize_method() != ResizeMethod::Stretch {
				output_targets
					.iter()
					.map(|target| target.dimensions.convert_aspect_ratio(cropped_dims.aspect_ratio()))
					.collect()
			} else {
				output_targets.iter().map(|target| target.dimensions).collect()
			};

		if resize_targets.is_empty() {
			return Err(ResizeError::NoValidResizeTargets);
		}

		Ok(Self {
			resizer: fr::Resizer::new(match output.resize_algorithm() {
				ResizeAlgorithm::Nearest => fr::ResizeAlg::Nearest,
				ResizeAlgorithm::Box => fr::ResizeAlg::Convolution(fr::FilterType::Box),
				ResizeAlgorithm::Bilinear => fr::ResizeAlg::Convolution(fr::FilterType::Bilinear),
				ResizeAlgorithm::Hamming => fr::ResizeAlg::Convolution(fr::FilterType::Hamming),
				ResizeAlgorithm::CatmullRom => fr::ResizeAlg::Convolution(fr::FilterType::CatmullRom),
				ResizeAlgorithm::Mitchell => fr::ResizeAlg::Convolution(fr::FilterType::Mitchell),
				ResizeAlgorithm::Lanczos3 => fr::ResizeAlg::Convolution(fr::FilterType::Lanczos3),
			}),
			input_dims,
			cropped_dims,
			crop: output.crop.as_ref().map(|crop| CropBox {
				left: crop.x as f64,
				top: crop.y as f64,
				width: crop.width as f64,
				height: crop.height as f64,
			}),
			resize_method: output.resize_method(),
			resize_dims: resize_targets,
			outputs: output_targets,
			output_frames,
			disable_resize_chaining: output.disable_resize_chaining,
		})
	}

	pub fn outputs(&self) -> &[ResizeOutputTarget] {
		&self.outputs
	}

	/// Resize the given frame to the target size, returning a reference to the
	/// resized frame. After this function returns original frame can be
	/// dropped, the returned frame is valid for the lifetime of the Resizer.
	pub fn resize(&mut self, frame: FrameRef) -> Result<&[Frame], ResizeError> {
		if frame.image.width() != self.input_dims.width || frame.image.height() != self.input_dims.height {
			return Err(ResizeError::MismatchedDimensions);
		}

		let input_image = fr::Image::from_slice_u8(
			NonZeroU32::new(frame.image.width() as u32).unwrap(),
			NonZeroU32::new(frame.image.height() as u32).unwrap(),
			// Safety: The input_image type is non_mut which disallows mutable actions on the underlying buffer.
			unsafe {
				let buf = frame.image.buf().as_bytes();
				std::slice::from_raw_parts_mut(buf.as_ptr() as *mut u8, buf.len())
			},
			fr::PixelType::U8x4,
		)?;

		let resize_dims = self.resize_dims.iter().rev().copied();
		let output_dims = self.outputs.iter().rev().map(|output| output.dimensions);
		let output_frames = self.output_frames.iter_mut().rev();

		let mut previous_image = ImageRef::Ref((&input_image, self.crop));

		for ((resize_dims, output_dims), output_frame) in resize_dims.zip(output_dims).zip(output_frames) {
			output_frame.duration_ts = frame.duration_ts;

			let mut target_image = fr::Image::from_slice_u8(
				NonZeroU32::new(output_dims.width as u32).unwrap(),
				NonZeroU32::new(output_dims.height as u32).unwrap(),
				output_frame.image.buf_mut().as_mut_slice().as_bytes_mut(),
				fr::PixelType::U8x4,
			)?;

			let mut view = previous_image.view();
			if let Some(crop) = previous_image.crop() {
				view.set_crop_box(crop)?;
			}

			let (mut target_view, target_crop) = if resize_dims != output_dims {
				let (left, top, width, height) = resize_method_to_crop_dims(self.resize_method, output_dims, resize_dims)?;
				(
					target_image.view_mut().crop(left, top, width, height)?,
					Some(CropBox {
						left: left as f64,
						top: top as f64,
						width: width.get() as f64,
						height: height.get() as f64,
					}),
				)
			} else {
				(target_image.view_mut(), None)
			};

			self.resizer.resize(&view, &mut target_view)?;

			// If we are upscaling then we dont want to downscale from an upscaled image.
			// Or if the user has explicitly disabled the resize chain.
			if self.disable_resize_chaining || self.cropped_dims < resize_dims {
				previous_image = ImageRef::Ref((&input_image, self.crop));
			} else {
				previous_image = ImageRef::Owned((target_image, target_crop));
			}
		}

		Ok(&self.output_frames)
	}
}

fn resize_method_to_crop_dims(
	resize_method: ResizeMethod,
	padded_dims: Dimensions,
	target_dims: Dimensions,
) -> Result<(u32, u32, NonZeroU32, NonZeroU32), ResizeError> {
	let check = |cmp: bool, msg: &'static str| if cmp { Ok(()) } else { Err(ResizeError::Internal(msg)) };

	check(padded_dims.width >= target_dims.width, "padded width less then target width")?;
	check(
		padded_dims.height >= target_dims.height,
		"padded height less then target height",
	)?;

	let center_x = (padded_dims.width - target_dims.width) as u32 / 2;
	let center_y = (padded_dims.height - target_dims.height) as u32 / 2;
	let left = (padded_dims.width - target_dims.width) as u32;
	let top = (padded_dims.height - target_dims.height) as u32;
	let bottom = 0;
	let right = 0;
	let zero = 0;

	let width = NonZeroU32::new(target_dims.width as u32).ok_or(ResizeError::Internal("target width 0"))?;
	let height = NonZeroU32::new(target_dims.height as u32).ok_or(ResizeError::Internal("target height 0"))?;

	match resize_method {
		ResizeMethod::Fit => Err(ResizeError::Internal("fit should never be called here")),
		ResizeMethod::Stretch => Err(ResizeError::Internal("stretch should never be called here")),
		ResizeMethod::PadLeft => {
			check(
				target_dims.width != padded_dims.width,
				"pad left should only be called for width padding",
			)?;
			Ok((left, zero, width, height))
		}
		ResizeMethod::PadRight => {
			check(
				target_dims.height != padded_dims.height,
				"pad right should only be called for height padding",
			)?;
			Ok((right, zero, width, height))
		}
		ResizeMethod::PadBottom => {
			check(
				target_dims.width != padded_dims.width,
				"pad bottom should only be called for height padding",
			)?;
			Ok((zero, bottom, width, height))
		}
		ResizeMethod::PadTop => {
			check(
				target_dims.width != padded_dims.width,
				"pad top should only be called for height padding",
			)?;
			Ok((zero, top, width, height))
		}
		ResizeMethod::PadBottomCenter => Ok((center_x, bottom, width, height)),
		ResizeMethod::PadTopCenter => Ok((center_x, top, width, height)),
		ResizeMethod::PadBottomLeft => Ok((left, bottom, width, height)),
		ResizeMethod::PadBottomRight => Ok((right, bottom, width, height)),
		ResizeMethod::PadTopLeft => Ok((left, top, width, height)),
		ResizeMethod::PadTopRight => Ok((right, top, width, height)),
		ResizeMethod::PadCenter => Ok((center_x, center_y, width, height)),
		ResizeMethod::PadCenterLeft => Ok((left, center_y, width, height)),
		ResizeMethod::PadCenterRight => Ok((right, center_y, width, height)),
	}
}
