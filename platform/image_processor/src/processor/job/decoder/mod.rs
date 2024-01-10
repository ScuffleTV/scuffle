use std::borrow::Cow;

use file_format::FileFormat;

use super::frame::Frame;
use crate::database::Job;
use crate::processor::error::{ProcessorError, Result};

mod ffmpeg;
mod libavif;
mod libwebp;

#[derive(Debug, Clone, Copy)]
pub enum DecoderBackend {
	Ffmpeg,
	LibWebp,
	LibAvif,
}

impl DecoderBackend {
	pub const fn from_format(format: FileFormat) -> Result<Self> {
		match format {
			FileFormat::Webp => Ok(Self::LibWebp), // .webp
			FileFormat::Av1ImageFileFormat  // .avif
			| FileFormat::Av1ImageFileFormatSequence => Ok(Self::LibAvif), // .avifs
			FileFormat::GraphicsInterchangeFormat // .gif
			| FileFormat::PortableNetworkGraphics // .png
			| FileFormat::AnimatedPortableNetworkGraphics // .apng
			| FileFormat::JpegLs // .jls
			| FileFormat::JointPhotographicExpertsGroup // .jpg
			| FileFormat::JpegXl // .jxl
			| FileFormat::WindowsBitmap // .bmp
			| FileFormat::HighEfficiencyImageCoding // .heic
			| FileFormat::HighEfficiencyImageCodingSequence // .heics
			| FileFormat::HighEfficiencyImageFileFormat // .heif
			| FileFormat::HighEfficiencyImageFileFormatSequence // .heifs
			| FileFormat::Mpeg4Part14 // .mp4
			| FileFormat::Mpeg4Part14Video // .mp4v
			| FileFormat::FlashVideo // .flv
			| FileFormat::Matroska3dVideo // .mk3d
			| FileFormat::MatroskaVideo // .mkv
			| FileFormat::AudioVideoInterleave // .avi
			| FileFormat::AppleQuicktime // .mov
			| FileFormat::Webm // .webm
			| FileFormat::BdavMpeg2TransportStream // .m2ts
			| FileFormat::Mpeg2TransportStream => Ok(Self::Ffmpeg), // .ts
			_ => Err(ProcessorError::UnsupportedInputFormat(format)),
		}
	}

	pub fn build<'a>(&self, job: &Job, data: Cow<'a, [u8]>) -> Result<AnyDecoder<'a>> {
		match self {
			Self::Ffmpeg => Ok(AnyDecoder::Ffmpeg(ffmpeg::FfmpegDecoder::new(job, data)?)),
			Self::LibAvif => Ok(AnyDecoder::LibAvif(libavif::AvifDecoder::new(job, data)?)),
			Self::LibWebp => Ok(AnyDecoder::LibWebp(libwebp::WebpDecoder::new(job, data)?)),
		}
	}
}

pub enum AnyDecoder<'a> {
	Ffmpeg(ffmpeg::FfmpegDecoder<'a>),
	LibAvif(libavif::AvifDecoder<'a>),
	LibWebp(libwebp::WebpDecoder<'a>),
}

pub trait Decoder {
	fn backend(&self) -> DecoderBackend;
	fn info(&self) -> DecoderInfo;
	fn decode(&mut self) -> Result<Option<Frame>>;
}

#[derive(Debug, Clone, Copy)]
pub struct DecoderInfo {
	pub width: usize,
	pub height: usize,
	pub loop_count: LoopCount,
	pub frame_count: usize,
	pub timescale: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopCount {
	Infinite,
	Finite(usize),
}

impl Decoder for AnyDecoder<'_> {
	fn backend(&self) -> DecoderBackend {
		match self {
			Self::Ffmpeg(decoder) => decoder.backend(),
			Self::LibAvif(decoder) => decoder.backend(),
			Self::LibWebp(decoder) => decoder.backend(),
		}
	}

	fn info(&self) -> DecoderInfo {
		match self {
			Self::Ffmpeg(decoder) => decoder.info(),
			Self::LibAvif(decoder) => decoder.info(),
			Self::LibWebp(decoder) => decoder.info(),
		}
	}

	fn decode(&mut self) -> Result<Option<Frame>> {
		match self {
			Self::Ffmpeg(decoder) => decoder.decode(),
			Self::LibAvif(decoder) => decoder.decode(),
			Self::LibWebp(decoder) => decoder.decode(),
		}
	}
}
