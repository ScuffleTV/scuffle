use std::borrow::Cow;

use anyhow::{anyhow, Context as _};
use imgref::Img;

use super::{Decoder, DecoderBackend, DecoderInfo, LoopCount};
use crate::database::Job;
use crate::processor::error::{ProcessorError, Result};
use crate::processor::job::frame::Frame;

pub struct FfmpegDecoder<'data> {
	input: ffmpeg::io::Input<std::io::Cursor<Cow<'data, [u8]>>>,
	decoder: ffmpeg::decoder::VideoDecoder,
	scaler: ffmpeg::scalar::Scalar,
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

impl<'data> FfmpegDecoder<'data> {
	pub fn new(job: &Job, data: Cow<'data, [u8]>) -> Result<Self> {
		let input = ffmpeg::io::Input::seekable(std::io::Cursor::new(data))
			.context("input")
			.map_err(ProcessorError::FfmpegDecode)?;

		let input_stream = input
			.streams()
			.best(ffmpeg::ffi::AVMediaType::AVMEDIA_TYPE_VIDEO)
			.ok_or_else(|| ProcessorError::FfmpegDecode(anyhow!("no video stream")))?;

		let input_stream_index = input_stream.index();

		let input_stream_time_base = input_stream.time_base();
		let input_stream_duration = input_stream.duration().unwrap_or(0);
		let input_stream_frames = input_stream
			.nb_frames()
			.ok_or_else(|| ProcessorError::FfmpegDecode(anyhow!("no frame count")))?.max(1);

		if input_stream_time_base.den == 0 || input_stream_time_base.num == 0 {
			return Err(ProcessorError::FfmpegDecode(anyhow!("stream time base is 0")));
		}

		let decoder = match ffmpeg::decoder::Decoder::new(&input_stream)
			.context("video decoder")
			.map_err(ProcessorError::FfmpegDecode)?
		{
			ffmpeg::decoder::Decoder::Video(decoder) => decoder,
			_ => return Err(ProcessorError::FfmpegDecode(anyhow!("not a video decoder"))),
		};

		let max_input_width = job.task.limits.as_ref().map(|l| l.max_input_width).unwrap_or(0) as i32;
		let max_input_height = job.task.limits.as_ref().map(|l| l.max_input_height).unwrap_or(0) as i32;
		let max_input_frame_count = job.task.limits.as_ref().map(|l| l.max_input_frame_count).unwrap_or(0) as i32;
		let max_input_duration_ms = job.task.limits.as_ref().map(|l| l.max_input_duration_ms).unwrap_or(0) as i32;

		if max_input_width > 0 && decoder.width() > max_input_width {
			return Err(ProcessorError::FfmpegDecode(anyhow!("input width exceeds limit")));
		}

		if max_input_height > 0 && decoder.height() > max_input_height {
			return Err(ProcessorError::FfmpegDecode(anyhow!("input height exceeds limit")));
		}

		if max_input_frame_count > 0 && input_stream_frames > max_input_frame_count as i64 {
			return Err(ProcessorError::FfmpegDecode(anyhow!("input frame count exceeds limit")));
		}

		if max_input_duration_ms > 0
			&& (input_stream_duration * input_stream_time_base.den as i64 * 1000) / input_stream_time_base.num as i64
				> max_input_duration_ms as i64
		{
			return Err(ProcessorError::FfmpegDecode(anyhow!("input duration exceeds limit")));
		}

		let scaler = ffmpeg::scalar::Scalar::new(
			decoder.width(),
			decoder.height(),
			decoder.pixel_format(),
			decoder.width(),
			decoder.height(),
			ffmpeg::ffi::AVPixelFormat::AV_PIX_FMT_RGBA,
		)
		.context("scaler")
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
					.map_err(ProcessorError::FfmpegDecode)?;

				if let Some(packet) = packet {
					self.decoder.send_packet(&packet).context("send packet").map_err(|err| {
						self.done = true;
						ProcessorError::FfmpegDecode(err)
					})?;
				} else {
					self.decoder.send_eof().context("send eof").map_err(|err| {
						self.done = true;
						ProcessorError::FfmpegDecode(err)
					})?;
					self.eof = true;
				}

				self.send_packet = false;
			}

			let frame = self.decoder.receive_frame().context("receive frame").map_err(|err| {
				self.done = true;
				ProcessorError::FfmpegDecode(err)
			})?;

			if let Some(frame) = frame {
				let frame = self.scaler.process(&frame).context("scaler run").map_err(|err| {
					self.done = true;
					ProcessorError::FfmpegDecode(err)
				})?;

				let data = cast_bytes_to_rgba(frame.data(0).unwrap()).to_vec();

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
