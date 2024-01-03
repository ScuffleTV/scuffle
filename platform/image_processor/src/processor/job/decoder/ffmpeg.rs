use std::borrow::Cow;

use anyhow::{anyhow, Context as _};
use ffmpeg_next::format::Pixel;
use ffmpeg_next::frame::Video;
use ffmpeg_next::software::scaling::{Context, Flags};
use imgref::ImgRef;

use super::{Decoder, DecoderBackend, DecoderInfo, FrameRef, LoopCount};
use crate::database::Job;
use crate::processor::error::{ProcessorError, Result};
use crate::processor::job::frame::FrameCow;

static FFMPEG: std::sync::Once = std::sync::Once::new();

pub fn init() {
	FFMPEG.call_once(|| {
		ffmpeg_next::init().expect("ffmpeg init failed");
	});
}

pub struct FfmpegDecoder<'data> {
	input: ffmpeg_next_io::Input<std::io::Cursor<Cow<'data, [u8]>>>,
	decoder: ffmpeg_next::codec::decoder::video::Video,
	scaler: ffmpeg_next::software::scaling::Context,
	info: DecoderInfo,
	input_stream_index: usize,
	average_frame_duration_ts: u64,
	duration_ts: i64,
	frame: Video,
	frame_rgba: Video,
	frame_rgba_2: Video,
	frame_buffer: Option<i64>,
	send_packet: bool,
	eof: bool,
	done: bool,
}

const fn cast_bytes_to_rgba(bytes: &[u8]) -> &[rgb::RGBA8] {
	unsafe { std::slice::from_raw_parts(bytes.as_ptr() as *const _, bytes.len() / 4) }
}

impl<'data> FfmpegDecoder<'data> {
	pub fn new(job: &Job, data: Cow<'data, [u8]>) -> Result<Self> {
		init();

		let input = ffmpeg_next_io::Input::seekable(std::io::Cursor::new(data))
			.map_err(|(_, err)| err)
			.context("input")
			.map_err(ProcessorError::FfmpegDecode)?;

		let input_stream = input
			.streams()
			.best(ffmpeg_next::media::Type::Video)
			.ok_or_else(|| ProcessorError::FfmpegDecode(anyhow!("no video stream")))?;

		let input_stream_index = input_stream.index();

		let input_stream_duration = input_stream.duration();
		let input_stream_time_base = input_stream.time_base();

		if input_stream_duration == 0 {
			return Err(ProcessorError::FfmpegDecode(anyhow!("stream duration is 0")));
		}

		if input_stream_time_base.0 == 0 || input_stream_time_base.1 == 0 {
			return Err(ProcessorError::FfmpegDecode(anyhow!("stream time base is 0")));
		}

		let context_decoder = ffmpeg_next::codec::context::Context::from_parameters(input_stream.parameters())
			.context("context decoder")
			.map_err(ProcessorError::FfmpegDecode)?;

		let decoder = context_decoder
			.decoder()
			.video()
			.context("decoder")
			.map_err(ProcessorError::FfmpegDecode)?;

		let max_input_width = job.task.limits.as_ref().map(|l| l.max_input_width).unwrap_or(0);
		let max_input_height = job.task.limits.as_ref().map(|l| l.max_input_height).unwrap_or(0);
		let max_input_frame_count = job.task.limits.as_ref().map(|l| l.max_input_frame_count).unwrap_or(0);
		let max_input_duration_ms = job.task.limits.as_ref().map(|l| l.max_input_duration_ms).unwrap_or(0);

		if max_input_width > 0 && decoder.width() > max_input_width {
			return Err(ProcessorError::FfmpegDecode(anyhow!("input width exceeds limit")));
		}

		if max_input_height > 0 && decoder.height() > max_input_height {
			return Err(ProcessorError::FfmpegDecode(anyhow!("input height exceeds limit")));
		}

		if max_input_frame_count > 0 && input_stream.frames() > max_input_frame_count as i64 {
			return Err(ProcessorError::FfmpegDecode(anyhow!("input frame count exceeds limit")));
		}

		if max_input_duration_ms > 0
			&& (input_stream.duration() * input_stream_time_base.1 as i64 * 1000) / input_stream_time_base.0 as i64
				> max_input_duration_ms as i64
		{
			return Err(ProcessorError::FfmpegDecode(anyhow!("input duration exceeds limit")));
		}

		let scaler = Context::get(
			decoder.format(),
			decoder.width(),
			decoder.height(),
			Pixel::RGBA,
			decoder.width(),
			decoder.height(),
			Flags::BILINEAR,
		)
		.context("scaler")
		.map_err(ProcessorError::FfmpegDecode)?;

		Ok(Self {
			info: DecoderInfo {
				width: decoder.width() as usize,
				height: decoder.height() as usize,
				frame_count: input_stream.frames() as usize,
				// TODO: Support loop count from ffmpeg.
				loop_count: LoopCount::Infinite,
				timescale: input_stream_time_base.1 as u64,
			},
			duration_ts: input_stream_duration,
			average_frame_duration_ts: (input_stream_duration / input_stream.frames() as i64) as u64,
			input,
			scaler,
			decoder,
			input_stream_index,
			done: false,
			eof: false,
			frame: Video::empty(),
			frame_buffer: None,
			frame_rgba: Video::empty(),
			frame_rgba_2: Video::empty(),
			send_packet: true,
		})
	}
}

impl Decoder for FfmpegDecoder<'_> {
	fn backend(&self) -> DecoderBackend {
		DecoderBackend::Ffmpeg
	}

	fn decode(&mut self) -> Result<Option<FrameCow<'_>>> {
		if self.done {
			return Ok(None);
		}

		loop {
			if self.send_packet && !self.eof {
				let packet = self.input.packets().find_map(|(stream, packet)| {
					if stream.index() == self.input_stream_index {
						Some(packet)
					} else {
						None
					}
				});

				if let Some(packet) = packet {
					if let Err(err) = self.decoder.send_packet(&packet).context("send packet") {
						self.done = true;
						return Err(ProcessorError::FfmpegDecode(err));
					}
				} else {
					if let Err(err) = self.decoder.send_eof().context("send eof") {
						self.done = true;
						return Err(ProcessorError::FfmpegDecode(err));
					}

					self.eof = true;
				}

				self.send_packet = false;
			}

			if self.decoder.receive_frame(&mut self.frame).is_ok() {
				if let Err(err) = self.scaler.run(&self.frame, &mut self.frame_rgba).context("scaler run") {
					self.done = true;
					return Err(ProcessorError::FfmpegDecode(err));
				}

				if let Some(pts) = self.frame.pts() {
					std::mem::swap(&mut self.frame_rgba, &mut self.frame_rgba_2);
					let old_pts = self.frame_buffer.replace(pts);

					if let Some(old_pts) = old_pts {
						self.frame_buffer = Some(pts);
						return Ok(Some(
							FrameRef {
								image: ImgRef::new(
									cast_bytes_to_rgba(self.frame_rgba.data(0)),
									self.info.width,
									self.info.height,
								),
								duration_ts: (pts - old_pts) as u64,
							}
							.into(),
						));
					}
				} else {
					return Ok(Some(
						FrameRef {
							image: ImgRef::new(
								cast_bytes_to_rgba(self.frame_rgba.data(0)),
								self.info.width,
								self.info.height,
							),
							duration_ts: self.average_frame_duration_ts,
						}
						.into(),
					));
				}
			} else if self.eof {
				self.done = true;

				if let Some(pts) = self.frame_buffer.take() {
					return Ok(Some(
						FrameRef {
							image: ImgRef::new(
								cast_bytes_to_rgba(self.frame_rgba_2.data(0)),
								self.info.width,
								self.info.height,
							),
							duration_ts: (self.duration_ts - pts) as u64,
						}
						.into(),
					));
				}

				return Ok(None);
			} else {
				self.send_packet = true;
			}
		}
	}

	fn info(&self) -> DecoderInfo {
		self.info
	}
}
