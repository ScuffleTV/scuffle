use anyhow::Context;
use rgb::ComponentBytes;

use super::{Encoder, EncoderFrontend, EncoderInfo, EncoderSettings};
use crate::processor::error::{ProcessorError, Result};
use crate::processor::job::frame::Frame;

pub struct PngEncoder {
	result: Option<Vec<u8>>,
	info: EncoderInfo,
}

impl PngEncoder {
	pub fn new(settings: EncoderSettings) -> Result<Self> {
		Ok(Self {
			result: None,
			info: EncoderInfo {
				duration: 0,
				frame_count: 0,
				frontend: EncoderFrontend::Png,
				height: 0,
				loop_count: settings.loop_count,
				timescale: settings.timescale,
				width: 0,
			},
		})
	}
}

impl Encoder for PngEncoder {
	fn info(&self) -> EncoderInfo {
		self.info
	}

	fn add_frame(&mut self, frame: &Frame) -> Result<()> {
		let _abort_guard = scuffle_utils::task::AbortGuard::new();

		if self.result.is_some() {
			return Err(ProcessorError::PngEncode(anyhow::anyhow!("encoder already finished")));
		}

		self.info.height = frame.image.height();
		self.info.width = frame.image.width();
		self.info.frame_count += 1;

		let mut result = Vec::new();

		let mut encoder = png::Encoder::new(&mut result, frame.image.width() as u32, frame.image.height() as u32);

		assert!(
			frame.image.buf().as_bytes().len() == frame.image.width() * frame.image.height() * 4,
			"image buffer size mismatch"
		);

		encoder.set_color(png::ColorType::Rgba);
		encoder.set_depth(png::BitDepth::Eight);
		encoder
			.write_header()
			.context("failed to write png header")
			.map_err(ProcessorError::PngEncode)?
			.write_image_data(frame.image.buf().as_bytes())
			.context("failed to write png data")
			.map_err(ProcessorError::PngEncode)?;

		self.result = Some(result);

		Ok(())
	}

	fn finish(self) -> Result<Vec<u8>> {
		self.result
			.ok_or_else(|| ProcessorError::PngEncode(anyhow::anyhow!("encoder not finished")))
	}
}
