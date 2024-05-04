use std::borrow::Cow;

use anyhow::{anyhow, Context as _};
use imgref::Img;
use rgb::RGBA8;

use super::{Decoder, DecoderBackend, DecoderInfo, LoopCount};
use crate::database::Job;
use crate::processor::error::{DecoderError, ProcessorError, Result};
use crate::processor::job::frame::Frame;

pub struct FfmpegDecoder<'data> {
	input: scuffle_ffmpeg::io::Input<std::io::Cursor<Cow<'data, [u8]>>>,
	decoder: scuffle_ffmpeg::decoder::VideoDecoder,
	scaler: scuffle_ffmpeg::scalar::Scalar,
	info: DecoderInfo,
	input_stream_index: i32,
	average_frame_duration_ts: u64,
	send_packet: bool,
	eof: bool,
	done: bool,
}

const fn cast_bytes_to_rgba(bytes: &[u8]) -> &[rgb::RGBA8] {
	unsafe { std::slice::from_raw_parts(bytes.as_ptr() as *const _, bytes.len() / 4) }
}

static FFMPEG_LOGGING_INITIALIZED: std::sync::Once = std::sync::Once::new();

impl<'data> FfmpegDecoder<'data> {
	pub fn new(job: &Job, data: Cow<'data, [u8]>) -> Result<Self> {
		FFMPEG_LOGGING_INITIALIZED.call_once(|| {
			scuffle_ffmpeg::log::log_callback_tracing();
		});

		let input = scuffle_ffmpeg::io::Input::seekable(std::io::Cursor::new(data))
			.context("input")
			.map_err(DecoderError::Other)
			.map_err(ProcessorError::FfmpegDecode)?;

		let input_stream = input
			.streams()
			.best(scuffle_ffmpeg::ffi::AVMediaType::AVMEDIA_TYPE_VIDEO)
			.ok_or_else(|| ProcessorError::FfmpegDecode(DecoderError::Other(anyhow!("no video stream"))))?;

		let input_stream_index = input_stream.index();

		let input_stream_time_base = input_stream.time_base();
		let input_stream_duration = input_stream.duration().unwrap_or(0);
		let input_stream_frames = input_stream
			.nb_frames()
			.ok_or_else(|| ProcessorError::FfmpegDecode(DecoderError::Other(anyhow!("no frame count"))))?
			.max(1);

		if input_stream_time_base.den == 0 || input_stream_time_base.num == 0 {
			return Err(ProcessorError::FfmpegDecode(DecoderError::Other(anyhow!(
				"stream time base is 0"
			))));
		}

		let decoder = match scuffle_ffmpeg::decoder::Decoder::new(&input_stream)
			.context("video decoder")
			.map_err(DecoderError::Other)
			.map_err(ProcessorError::FfmpegDecode)?
		{
			scuffle_ffmpeg::decoder::Decoder::Video(decoder) => decoder,
			_ => {
				return Err(ProcessorError::FfmpegDecode(DecoderError::Other(anyhow!(
					"not a video decoder"
				))));
			}
		};

		let max_input_width = job.task.limits.as_ref().map(|l| l.max_input_width).unwrap_or(0) as i32;
		let max_input_height = job.task.limits.as_ref().map(|l| l.max_input_height).unwrap_or(0) as i32;
		let max_input_frame_count = job.task.limits.as_ref().map(|l| l.max_input_frame_count).unwrap_or(0) as i32;
		let max_input_duration_ms = job.task.limits.as_ref().map(|l| l.max_input_duration_ms).unwrap_or(0) as i32;

		if max_input_width > 0 && decoder.width() > max_input_width {
			return Err(ProcessorError::FfmpegDecode(DecoderError::TooWide(decoder.width())));
		}

		if max_input_height > 0 && decoder.height() > max_input_height {
			return Err(ProcessorError::FfmpegDecode(DecoderError::TooHigh(decoder.height())));
		}

		if max_input_frame_count > 0 && input_stream_frames > max_input_frame_count as i64 {
			return Err(ProcessorError::FfmpegDecode(DecoderError::TooManyFrames(input_stream_frames)));
		}

		// actual duration
		// = duration * (time_base.num / time_base.den) * 1000
		// = (duration * time_base.num * 1000) / time_base.den
		let duration =
			(input_stream_duration * input_stream_time_base.num as i64 * 1000) / input_stream_time_base.den as i64;
		if max_input_duration_ms > 0 && duration > max_input_duration_ms as i64 {
			return Err(ProcessorError::FfmpegDecode(DecoderError::TooLong(duration)));
		}

		let scaler = scuffle_ffmpeg::scalar::Scalar::new(
			decoder.width(),
			decoder.height(),
			decoder.pixel_format(),
			decoder.width(),
			decoder.height(),
			scuffle_ffmpeg::ffi::AVPixelFormat::AV_PIX_FMT_RGBA,
		)
		.context("scaler")
		.map_err(DecoderError::Other)
		.map_err(ProcessorError::FfmpegDecode)?;

		Ok(Self {
			info: DecoderInfo {
				width: decoder.width() as usize,
				height: decoder.height() as usize,
				frame_count: input_stream_frames as usize,
				// TODO: Support loop count from ffmpeg.
				loop_count: LoopCount::Infinite,
				timescale: input_stream_time_base.den as u64,
			},
			average_frame_duration_ts: (input_stream_duration / input_stream_frames) as u64,
			input,
			scaler,
			decoder,
			input_stream_index,
			done: false,
			eof: false,
			send_packet: true,
		})
	}
}

impl Decoder for FfmpegDecoder<'_> {
	fn backend(&self) -> DecoderBackend {
		DecoderBackend::Ffmpeg
	}

	fn decode(&mut self) -> Result<Option<Frame>> {
		if self.done {
			return Ok(None);
		}

		loop {
			if self.send_packet && !self.eof {
				let packet = self
					.input
					.packets()
					.find_map(|packet| match packet {
						Ok(packet) => {
							if packet.stream_index() == self.input_stream_index {
								Some(Ok(packet))
							} else {
								None
							}
						}
						Err(err) => {
							self.done = true;
							Some(Err(err))
						}
					})
					.transpose()
					.context("receive packet")
					.map_err(DecoderError::Other)
					.map_err(ProcessorError::FfmpegDecode)?;

				if let Some(packet) = packet {
					self.decoder.send_packet(&packet).context("send packet").map_err(|err| {
						self.done = true;
						ProcessorError::FfmpegDecode(DecoderError::Other(err))
					})?;
				} else {
					self.decoder.send_eof().context("send eof").map_err(|err| {
						self.done = true;
						ProcessorError::FfmpegDecode(DecoderError::Other(err))
					})?;
					self.eof = true;
				}

				self.send_packet = false;
			}

			let frame = self.decoder.receive_frame().context("receive frame").map_err(|err| {
				self.done = true;
				ProcessorError::FfmpegDecode(DecoderError::Other(err))
			})?;

			if let Some(frame) = frame {
				let frame = self.scaler.process(&frame).context("scaler run").map_err(|err| {
					self.done = true;
					ProcessorError::FfmpegDecode(DecoderError::Other(err))
				})?;

				let mut data = vec![RGBA8::default(); frame.width() * frame.height()];

				// The frame has padding, so we need to copy the data.
				let frame_data = frame.data(0).unwrap();
				let frame_linesize = frame.linesize(0).unwrap();

				if frame_linesize == frame.width() as i32 * 4 {
					// No padding, so we can just copy the data.
					data.copy_from_slice(cast_bytes_to_rgba(frame_data));
				} else {
					// The frame has padding, so we need to copy the data.
					for (i, row) in data.chunks_exact_mut(frame.width()).enumerate() {
						let row_data = &frame_data[i * frame_linesize as usize..][..frame.width() * 4];
						row.copy_from_slice(cast_bytes_to_rgba(row_data));
					}
				}

				return Ok(Some(Frame {
					image: Img::new(data, self.info.width, self.info.height),
					duration_ts: self.average_frame_duration_ts,
				}));
			} else if self.eof {
				self.done = true;
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
