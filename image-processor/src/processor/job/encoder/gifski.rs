use anyhow::Context;
use scuffle_utils::task::Task;

use super::{Encoder, EncoderFrontend, EncoderInfo, EncoderSettings};
use crate::processor::error::{ProcessorError, Result};
use crate::processor::job::decoder::LoopCount;
use crate::processor::job::frame::Frame;

pub struct GifskiEncoder {
	collector: gifski::Collector,
	writer: Task<Result<Vec<u8>>>,
	info: EncoderInfo,
}

impl GifskiEncoder {
	pub fn new(settings: EncoderSettings) -> Result<Self> {
		let (collector, writer) = gifski::new(gifski::Settings {
			repeat: match settings.loop_count {
				LoopCount::Infinite => gifski::Repeat::Infinite,
				LoopCount::Finite(count) => gifski::Repeat::Finite(count as u16),
			},
			fast: settings.fast,
			..Default::default()
		})
		.context("failed to create gifski encoder")
		.map_err(ProcessorError::GifskiEncode)?;

		Ok(Self {
			collector,
			writer: Task::spawn("gifski writer", move || {
				let mut buffer = Vec::new();
				writer
					.write(&mut buffer, &mut gifski::progress::NoProgress {})
					.context("failed to write gifski output")
					.map_err(ProcessorError::GifskiEncode)?;
				Ok(buffer)
			}),
			info: EncoderInfo {
				duration: 0,
				frame_count: 0,
				frontend: EncoderFrontend::Gifski,
				height: 0,
				loop_count: settings.loop_count,
				timescale: settings.timescale,
				width: 0,
			},
		})
	}

	fn duration(&mut self, duration: u64) -> f64 {
		self.info.duration += duration;
		self.info.duration as f64 / self.info.timescale as f64
	}
}

impl Encoder for GifskiEncoder {
	fn info(&self) -> EncoderInfo {
		self.info
	}

	fn add_frame(&mut self, frame: &Frame) -> Result<()> {
		let _abort_guard = scuffle_utils::task::AbortGuard::new();

		let frame = frame.to_owned();
		self.info.height = frame.image.height();
		self.info.width = frame.image.width();
		let duration = self.duration(frame.duration_ts);
		self.collector
			.add_frame_rgba(self.info.frame_count, frame.image, duration)
			.context("failed to add frame to gifski")
			.map_err(ProcessorError::GifskiEncode)?;
		self.info.frame_count += 1;
		Ok(())
	}

	fn finish(self) -> Result<Vec<u8>> {
		let _abort_guard = scuffle_utils::task::AbortGuard::new();

		drop(self.collector);

		self.writer
			.join()
			.map_err(|err| anyhow::anyhow!("failed to join gifski thread: {:?}", err))
			.map_err(ProcessorError::GifskiEncode)?
	}
}
