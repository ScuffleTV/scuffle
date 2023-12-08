use super::decoder::LoopCount;
use super::frame::FrameRef;
use crate::processor::error::Result;

mod gifski;
mod libavif;
mod libwebp;
mod png;

#[derive(Debug, Clone, Copy)]
pub enum EncoderFrontend {
	Gifski,
	Png,
	LibWebp,
	LibAvif,
}

#[derive(Debug, Clone, Copy)]
pub struct EncoderSettings {
	pub fast: bool,
	pub loop_count: LoopCount,
	pub timescale: u64,
	pub static_image: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct EncoderInfo {
	pub frontend: EncoderFrontend,
	pub width: usize,
	pub height: usize,
	pub loop_count: LoopCount,
	pub timescale: u64,
	pub duration: u64,
	pub frame_count: usize,
}

impl EncoderFrontend {
	pub fn build(&self, settings: EncoderSettings) -> Result<AnyEncoder> {
		match self {
			Self::Png => Ok(AnyEncoder::Png(png::PngEncoder::new(settings)?)),
			Self::Gifski => Ok(AnyEncoder::Gifski(gifski::GifskiEncoder::new(settings)?)),
			Self::LibAvif => Ok(AnyEncoder::LibAvif(libavif::AvifEncoder::new(settings)?)),
			Self::LibWebp => Ok(AnyEncoder::LibWebp(libwebp::WebpEncoder::new(settings)?)),
		}
	}

	pub const fn extension(&self) -> &'static str {
		match self {
			Self::Png => "png",
			Self::Gifski => "gif",
			Self::LibAvif => "avif",
			Self::LibWebp => "webp",
		}
	}
}

pub enum AnyEncoder {
	Gifski(gifski::GifskiEncoder),
	Png(png::PngEncoder),
	LibAvif(libavif::AvifEncoder),
	LibWebp(libwebp::WebpEncoder),
}

pub trait Encoder {
	fn info(&self) -> EncoderInfo;
	fn add_frame(&mut self, frame: FrameRef<'_>) -> Result<()>;
	fn finish(self) -> Result<Vec<u8>>;
}

impl Encoder for AnyEncoder {
	fn info(&self) -> EncoderInfo {
		match self {
			Self::Gifski(encoder) => encoder.info(),
			Self::Png(encoder) => encoder.info(),
			Self::LibAvif(encoder) => encoder.info(),
			Self::LibWebp(encoder) => encoder.info(),
		}
	}

	fn add_frame(&mut self, frame: FrameRef<'_>) -> Result<()> {
		match self {
			Self::Gifski(encoder) => encoder.add_frame(frame),
			Self::Png(encoder) => encoder.add_frame(frame),
			Self::LibAvif(encoder) => encoder.add_frame(frame),
			Self::LibWebp(encoder) => encoder.add_frame(frame),
		}
	}

	fn finish(self) -> Result<Vec<u8>> {
		match self {
			Self::Gifski(encoder) => encoder.finish(),
			Self::Png(encoder) => encoder.finish(),
			Self::LibAvif(encoder) => encoder.finish(),
			Self::LibWebp(encoder) => encoder.finish(),
		}
	}
}
