use ffmpeg_sys_next::*;

use crate::codec::EncoderCodec;
use crate::dict::Dictionary;
use crate::error::FfmpegError;
use crate::frame::Frame;
use crate::io::Output;
use crate::packet::Packet;
use crate::smart_object::SmartPtr;

pub struct Encoder {
	incoming_time_base: AVRational,
	outgoing_time_base: AVRational,
	encoder: SmartPtr<AVCodecContext>,
	stream_index: i32,
	average_duration: i64,
}

/// Safety: `Encoder` can be sent between threads.
unsafe impl Send for Encoder {}

pub enum EncoderSettings {
	Video {
		width: i32,
		height: i32,
		frame_rate: i32,
		gop_size: i32,
		qmax: i32,
		qmin: i32,
		pixel_format: AVPixelFormat,
		thread_count: i32,
	},
	Audio {},
}

impl Encoder {
	pub fn new<T>(
		codec: EncoderCodec,
		output: &mut Output<T>,
		incoming_time_base: AVRational,
		outgoing_time_base: AVRational,
		codec_options: &mut Dictionary,
		settings: EncoderSettings,
	) -> Result<Self, FfmpegError> {
		if codec.as_ptr().is_null() {
			return Err(FfmpegError::NoEncoder);
		}

		let global_header = output.flags() & AVFMT_GLOBALHEADER != 0;

		// Safety: `avcodec_alloc_context3` is safe to call, and the pointer returned is valid.
		let mut encoder =
			unsafe { SmartPtr::wrap_non_null(avcodec_alloc_context3(codec.as_ptr()), |ptr| avcodec_free_context(ptr)) }
				.ok_or(FfmpegError::Alloc)?;

		let mut ost = output.add_stream(None).ok_or(FfmpegError::NoStream)?;

		let encoder_mut = encoder.as_deref_mut_except();

		encoder_mut.time_base = incoming_time_base;
		encoder_mut.flags |= AV_CODEC_FLAG_CLOSED_GOP as i32 | AV_CODEC_FLAG_LOW_DELAY as i32;
		if global_header {
			encoder_mut.flags |= AV_CODEC_FLAG_GLOBAL_HEADER as i32;
		}

		encoder_mut.thread_type = FF_THREAD_SLICE;

		let average_duration = match settings {
			EncoderSettings::Video {
				width,
				height,
				frame_rate,
				pixel_format,
				gop_size,
				qmax,
				qmin,
				thread_count,
			} => {
				encoder_mut.height = height;
				encoder_mut.width = width;
				encoder_mut.pix_fmt = pixel_format;
				encoder_mut.sample_aspect_ratio = AVRational { num: 1, den: 1 };
				encoder_mut.framerate = AVRational { num: frame_rate, den: 1 };
				encoder_mut.thread_count = thread_count;
				encoder_mut.gop_size = gop_size;
				encoder_mut.qmax = qmax;
				encoder_mut.qmin = qmin;
				// encoder_mut.bit_rate = 2 * 1024 * 1024;
				// encoder_mut.rc_buffer_size = 4 * 1024 * 1024;
				// encoder_mut.rc_max_rate = 2 * 1024 * 1024;
				// encoder_mut.rc_min_rate = 2 * 1024 * 1024;
				encoder_mut.max_b_frames = 0;
				((outgoing_time_base.den / frame_rate) / outgoing_time_base.num) as i64
			}
			EncoderSettings::Audio {} => {
				todo!("audio encoder settings")
			}
		};

		// Safety: `avcodec_open2` is safe to call, 'encoder' and 'codec' and 'codec_options' are a valid pointers.
		let res = unsafe { avcodec_open2(encoder_mut, codec.as_ptr(), codec_options.as_mut_ptr_ref()) };
		if res < 0 {
			return Err(FfmpegError::Code(res.into()));
		}

		// Safety: `avcodec_parameters_from_context` is safe to call, 'ost' and 'encoder' are valid pointers.
		let ret = unsafe { avcodec_parameters_from_context((*ost.as_mut_ptr()).codecpar, encoder_mut) };
		if ret < 0 {
			return Err(FfmpegError::Code(ret.into()));
		}

		ost.set_time_base(outgoing_time_base);

		Ok(Self {
			incoming_time_base,
			outgoing_time_base,
			encoder,
			average_duration,
			stream_index: ost.index(),
		})
	}

	pub fn with_output<T>(
		codec: EncoderCodec,
		mut output: Output<T>,
		incoming_time_base: AVRational,
		outgoing_time_base: AVRational,
		codec_options: &mut Dictionary,
		settings: EncoderSettings,
		interleave: bool,
		muxer_options: Dictionary,
	) -> Result<EncoderWithOutput<T>, FfmpegError> {
		Ok(EncoderWithOutput {
			encoder: Encoder::new(
				codec,
				&mut output,
				incoming_time_base,
				outgoing_time_base,
				codec_options,
				settings,
			)?,
			output,
			interleave,
			muxer_headers_written: false,
			muxer_options,
			buffered_packet: None,
		})
	}

	pub fn send_eof(&mut self) -> Result<(), FfmpegError> {
		// Safety: `self.encoder` is a valid pointer.
		let ret = unsafe { avcodec_send_frame(self.encoder.as_mut_ptr(), std::ptr::null()) };
		if ret == 0 {
			Ok(())
		} else {
			Err(FfmpegError::Code(ret.into()))
		}
	}

	pub fn send_frame(&mut self, frame: &Frame) -> Result<(), FfmpegError> {
		// Safety: `self.encoder` and `frame` are valid pointers.
		let ret = unsafe { avcodec_send_frame(self.encoder.as_mut_ptr(), frame.as_ptr()) };
		if ret == 0 {
			Ok(())
		} else {
			Err(FfmpegError::Code(ret.into()))
		}
	}

	pub fn receive_packet(&mut self) -> Result<Option<Packet>, FfmpegError> {
		let mut packet = Packet::new()?;

		const AVERROR_EAGAIN: i32 = AVERROR(EAGAIN);

		// Safety: `self.encoder` and `packet` are valid pointers.
		let ret = unsafe { avcodec_receive_packet(self.encoder.as_mut_ptr(), packet.as_mut_ptr()) };
		match ret {
			AVERROR_EAGAIN | AVERROR_EOF => Ok(None),
			0 => {
				packet.rescale_timebase(self.incoming_time_base, self.outgoing_time_base);
				packet.set_stream_index(self.stream_index);
				Ok(Some(packet))
			}
			_ => Err(FfmpegError::Code(ret.into())),
		}
	}

	pub fn stream_index(&self) -> i32 {
		self.stream_index
	}

	pub fn incoming_time_base(&self) -> AVRational {
		self.incoming_time_base
	}

	pub fn outgoing_time_base(&self) -> AVRational {
		self.outgoing_time_base
	}
}

pub struct EncoderWithOutput<T> {
	encoder: Encoder,
	output: Output<T>,
	interleave: bool,
	muxer_headers_written: bool,
	muxer_options: Dictionary,
	buffered_packet: Option<Packet>,
}

impl<T> EncoderWithOutput<T> {
	pub fn send_eof(&mut self) -> Result<(), FfmpegError> {
		self.encoder.send_eof()?;
		self.handle_packets()?;

		if let Some(mut bufferd_packet) = self.buffered_packet.take() {
			bufferd_packet.set_duration(Some(self.average_duration));

			if self.interleave {
				self.output.write_interleaved_packet(bufferd_packet)?;
			} else {
				self.output.write_packet(&bufferd_packet)?;
			}
		}

		if !self.muxer_headers_written {
			self.output.write_header_with_options(&mut self.muxer_options)?;
			self.muxer_headers_written = true;
		}

		self.output.write_trailer()?;
		Ok(())
	}

	pub fn send_frame(&mut self, frame: &Frame) -> Result<(), FfmpegError> {
		self.encoder.send_frame(frame)?;
		self.handle_packets()?;
		Ok(())
	}

	pub fn handle_packets(&mut self) -> Result<(), FfmpegError> {
		while let Some(packet) = self.encoder.receive_packet()? {
			if !self.muxer_headers_written {
				self.output.write_header_with_options(&mut self.muxer_options)?;
				self.muxer_headers_written = true;
			}

			if let Some(mut bufferd_packet) = self.buffered_packet.take() {
				match ((packet.dts(), bufferd_packet.dts()), (packet.pts(), bufferd_packet.pts())) {
					((Some(packet_dts), Some(bufferd_dts)), _) => {
						bufferd_packet.set_duration(Some(packet_dts - bufferd_dts))
					}
					(_, (Some(packet_pts), Some(bufferd_pts))) => {
						bufferd_packet.set_duration(Some(packet_pts - bufferd_pts))
					}
					_ => bufferd_packet.set_duration(Some(self.encoder.average_duration)),
				}

				if self.interleave {
					self.output.write_interleaved_packet(bufferd_packet)?;
				} else {
					self.output.write_packet(&bufferd_packet)?;
				}
			}

			self.buffered_packet = Some(packet);
		}

		Ok(())
	}

	pub fn stream_index(&self) -> i32 {
		self.encoder.stream_index()
	}

	pub fn incoming_time_base(&self) -> AVRational {
		self.encoder.incoming_time_base()
	}

	pub fn outgoing_time_base(&self) -> AVRational {
		self.encoder.outgoing_time_base()
	}

	pub fn into_inner(self) -> Output<T> {
		self.output
	}
}

impl<T> std::ops::Deref for EncoderWithOutput<T> {
	type Target = Encoder;

	fn deref(&self) -> &Self::Target {
		&self.encoder
	}
}

impl<T> std::ops::DerefMut for EncoderWithOutput<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.encoder
	}
}
