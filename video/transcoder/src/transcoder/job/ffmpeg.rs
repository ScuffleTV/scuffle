use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Context;
use bytes::Bytes;
use ffmpeg::codec::EncoderCodec;
use ffmpeg::decoder::Decoder;
use ffmpeg::dict::Dictionary;
use ffmpeg::encoder::{AudioEncoderSettings, MuxerEncoder, MuxerSettings, VideoEncoderSettings};
use ffmpeg::error::FfmpegError;
use ffmpeg::ffi::{AVCodecID, AVMediaType, AVPictureType, AVRational};
use ffmpeg::io::channel::{ChannelCompatRecv as _, ChannelCompatSend as _};
use ffmpeg::io::OutputOptions;
use ffmpeg::stream::Stream;
use mp4::codec::{AudioCodec, VideoCodec};
use pb::scuffle::video::v1::types::{AudioConfig, VideoConfig};
use tokio::sync::mpsc;
use video_common::database::Rendition;

use crate::config::TranscoderConfig;
use crate::global::TranscoderGlobal;

const MP4_FLAGS: &str = "frag_keyframe+frag_every_frame+empty_moov+delay_moov+default_base_moof";

type ChannelCompatRecv = ffmpeg::io::channel::ChannelCompat<mpsc::Receiver<Bytes>>;
type ChannelCompatSend = ffmpeg::io::channel::ChannelCompat<mpsc::Sender<Vec<u8>>>;

type Input = ffmpeg::io::Input<ChannelCompatRecv>;
type Output = ffmpeg::io::Output<ChannelCompatSend>;
type VideoDecoder = ffmpeg::decoder::VideoDecoder;
type AudioDecoder = ffmpeg::decoder::AudioDecoder;
type Encoder = ffmpeg::encoder::MuxerEncoder<ChannelCompatSend>;
type Scalar = ffmpeg::scalar::Scalar;
type Limiter = ffmpeg::limiter::FrameRateLimiter;

fn muxer_options() -> Dictionary {
	Dictionary::builder().set("movflags", MP4_FLAGS).build()
}

pub struct Transcoder {
	input: Option<Input>,
	video_stream_index: i32,
	audio_stream_index: i32,
	video_decoder: VideoDecoder,
	audio_decoder: Option<AudioDecoder>,
	video_copies: Vec<Output>,
	audio_copies: Vec<Output>,
	scalars: Vec<Scalar>,
	limiters: Vec<Limiter>,
	video_encoders: Vec<Encoder>,
	audio_encoders: Vec<Encoder>,
}

fn codec_to_encoder_codec_options(
	config: &TranscoderConfig,
	codec: VideoCodec,
) -> anyhow::Result<(EncoderCodec, Dictionary)> {
	match codec {
		VideoCodec::Avc { level, profile, .. } => {
			let mut options = Dictionary::from(config.h264_encoder_options.clone());

			options
				.set(
					"profile",
					match profile {
						66 => "baseline",
						77 => "main",
						100 => "high",
						_ => {
							anyhow::bail!("invalid h264 profile: {profile}");
						}
					},
				)
				.context("failed to set h264 profile")?;

			options
				.set(
					"level",
					match level {
						30 => "3.0",
						31 => "3.1",
						32 => "3.2",
						40 => "4.0",
						41 => "4.1",
						42 => "4.2",
						50 => "5.0",
						51 => "5.1",
						52 => "5.2",
						60 => "6.0",
						61 => "6.1",
						62 => "6.2",
						_ => {
							anyhow::bail!("invalid avc level: {level}");
						}
					},
				)
				.context("failed to set h264 level")?;

			Ok((
				config
					.h264_encoder
					.as_ref()
					.map(|name| ffmpeg::codec::EncoderCodec::by_name(name))
					.unwrap_or_else(|| ffmpeg::codec::EncoderCodec::new(AVCodecID::AV_CODEC_ID_H264))
					.ok_or(FfmpegError::NoEncoder)
					.context("failed to find h264 encoder")?,
				options,
			))
		}
		VideoCodec::Av1 { .. } => {
			anyhow::bail!("av1 encoding is not supported");
		}
		VideoCodec::Hevc { .. } => {
			anyhow::bail!("hevc encoding is not supported");
		}
	}
}

fn handle_audio_packet(
	mut packet: ffmpeg::packet::Packet,
	copies: &mut [Output],
	decoder: Option<&mut AudioDecoder>,
	encoders: &mut [Encoder],
) -> anyhow::Result<()> {
	packet.set_pos(Some(-1));

	for copy in copies.iter_mut() {
		copy.write_interleaved_packet(packet.clone()).context("copy")?;
	}

	if let Some(decoder) = decoder {
		decoder.send_packet(&packet).context("decoder send packet")?;
		handle_audio_decoder(decoder, encoders).context("decoder")?;
	}

	Ok(())
}

fn handle_audio_decoder(decoder: &mut AudioDecoder, encoders: &mut [Encoder]) -> anyhow::Result<()> {
	while let Some(mut frame) = decoder.receive_frame().context("receive frame")? {
		frame.set_pict_type(AVPictureType::AV_PICTURE_TYPE_NONE);
		let frame_timestamp = frame.best_effort_timestamp();
		frame.set_pts(frame_timestamp);

		for encoder in encoders.iter_mut() {
			encoder.send_frame(&frame).context("encoder")?;
		}
	}

	Ok(())
}

fn handle_audio_eof(
	copies: &mut [Output],
	decoder: Option<&mut AudioDecoder>,
	encoders: &mut [Encoder],
) -> anyhow::Result<()> {
	for copy in copies.iter_mut() {
		copy.write_trailer().context("copy")?;
	}

	if let Some(decoder) = decoder {
		decoder.send_eof().context("decoder eof")?;
		handle_audio_decoder(decoder, encoders)?;
	}

	for encoder in encoders.iter_mut() {
		encoder.send_eof().context("encoder eof")?;
	}

	Ok(())
}

fn handle_video_packet(
	mut packet: ffmpeg::packet::Packet,
	copies: &mut [Output],
	decoder: &mut VideoDecoder,
	encoders: &mut [Encoder],
	scalars: &mut [Scalar],
	limiters: &mut [Limiter],
) -> anyhow::Result<()> {
	packet.set_pos(Some(-1));
	for copy in copies.iter_mut() {
		copy.write_interleaved_packet(packet.clone()).context("copy")?;
	}

	decoder.send_packet(&packet).context("decoder send")?;

	handle_video_decoder(decoder, encoders, scalars, limiters).context("decoder")?;

	Ok(())
}

fn handle_video_decoder(
	decoder: &mut VideoDecoder,
	encoders: &mut [Encoder],
	scalars: &mut [Scalar],
	limiters: &mut [Limiter],
) -> anyhow::Result<()> {
	while let Some(mut frame) = decoder.receive_frame().context("receive frame")? {
		frame.set_pict_type(AVPictureType::AV_PICTURE_TYPE_NONE);
		let frame_timestamp = frame.best_effort_timestamp();
		frame.set_pts(frame_timestamp);
		frame.set_format(decoder.pixel_format() as i32);

		let mut frames = vec![];
		for (scalar, limiter) in scalars.iter_mut().zip(limiters.iter_mut()) {
			if !limiter.limit(&frame) {
				break;
			}

			frames.push(scalar.process(frames.last().copied().unwrap_or(&frame.0)).context("scalar")?);
		}

		for (encoder, frame) in encoders.iter_mut().zip(frames) {
			encoder.send_frame(frame).context("encoder")?;
		}
	}

	Ok(())
}

fn handle_video_eof(
	copies: &mut [Output],
	decoder: &mut VideoDecoder,
	encoders: &mut [Encoder],
	scalars: &mut [Scalar],
	limiters: &mut [Limiter],
) -> anyhow::Result<()> {
	for copy in copies.iter_mut() {
		copy.write_trailer().context("copy")?;
	}

	decoder.send_eof().context("decoder eof")?;

	handle_video_decoder(decoder, encoders, scalars, limiters).context("decoder")?;

	for encoder in encoders.iter_mut() {
		encoder.send_eof().context("encoder eof")?;
	}

	Ok(())
}

impl Transcoder {
	pub fn new(
		global: Arc<impl TranscoderGlobal>,
		input: mpsc::Receiver<Bytes>,
		mut outputs: HashMap<Rendition, mpsc::Sender<Vec<u8>>>,
		mut video_configs: Vec<VideoConfig>,
		mut audio_outputs: Vec<AudioConfig>,
	) -> anyhow::Result<Self> {
		let ictx = ffmpeg::io::Input::new(input.into_compat()).context("failed to create input")?;

		let video_stream = ictx
			.streams()
			.best(AVMediaType::AVMEDIA_TYPE_VIDEO)
			.ok_or(FfmpegError::NoStream)
			.context("failed to find video stream")?;

		let audio_stream = ictx
			.streams()
			.best(AVMediaType::AVMEDIA_TYPE_AUDIO)
			.ok_or(FfmpegError::NoStream)
			.context("failed to find video stream")?;

		let mut this = Self {
			audio_stream_index: audio_stream.index(),
			video_stream_index: video_stream.index(),
			video_decoder: match ffmpeg::decoder::Decoder::new(&video_stream).context("failed to create h264 decoder")? {
				Decoder::Video(decoder) => decoder,
				_ => anyhow::bail!("expected video decoder"),
			},
			input: None,
			audio_decoder: None,
			video_copies: Vec::new(),
			audio_copies: Vec::new(),
			scalars: Vec::new(),
			limiters: Vec::new(),
			video_encoders: Vec::new(),
			audio_encoders: Vec::new(),
		};

		if audio_outputs.iter().any(|c| c.rendition() == Rendition::AudioSource.into()) {
			let sender = outputs
				.remove(&Rendition::AudioSource)
				.ok_or_else(|| anyhow::anyhow!("missing audio source output"))?;

			let mut output = ffmpeg::io::Output::new(
				sender.into_compat(),
				OutputOptions {
					format_name: Some("mp4"),
					..Default::default()
				},
			)
			.context("failed to create output")?;

			output.copy_stream(&audio_stream).context("failed to copy audio stream")?;
			output.write_header_with_options(&mut muxer_options())?;

			this.audio_copies.push(output);
		}

		if video_configs.iter().any(|c| c.rendition() == Rendition::VideoSource.into()) {
			let sender = outputs
				.remove(&Rendition::VideoSource)
				.ok_or_else(|| anyhow::anyhow!("missing video source output"))?;

			let mut output = ffmpeg::io::Output::new(
				sender.into_compat(),
				OutputOptions {
					format_name: Some("mp4"),
					..Default::default()
				},
			)
			.context("failed to create output")?;

			output.copy_stream(&video_stream).context("failed to copy video stream")?;
			output.write_header_with_options(&mut muxer_options())?;

			this.video_copies.push(output);
		}

		audio_outputs.retain(|c| c.rendition() != Rendition::AudioSource.into());
		video_configs.retain(|c| c.rendition() != Rendition::VideoSource.into());

		let config = global.config();

		if !video_configs.is_empty() {
			for video_config in video_configs {
				let codec = video_config
					.codec
					.parse()
					.map_err(|err| anyhow::anyhow!("failed to parse video codec: {err}"))?;

				let (encoder_codec, encoder_options) = codec_to_encoder_codec_options(config, codec)?;

				let sender = outputs
					.remove(&Rendition::from(video_config.rendition()))
					.ok_or_else(|| anyhow::anyhow!("missing video output"))?;
				this.setup_video_encoder(sender, &video_config, &video_stream, encoder_codec, encoder_options)?;
			}
		}

		if !audio_outputs.is_empty() {
			this.audio_decoder = Some(
				match ffmpeg::decoder::Decoder::new(&audio_stream).context("failed to create aac decoder")? {
					Decoder::Audio(decoder) => decoder,
					_ => anyhow::bail!("expected audio decoder"),
				},
			);

			for audio_config in audio_outputs {
				let codec = audio_config
					.codec
					.parse()
					.map_err(|err| anyhow::anyhow!("failed to parse audio codec: {err}"))?;

				let (encoder_codec, encoder_options) = match codec {
					AudioCodec::Aac { object_type } => {
						let codec = ffmpeg::codec::EncoderCodec::by_name("libfdk_aac")
							.or_else(|| ffmpeg::codec::EncoderCodec::new(AVCodecID::AV_CODEC_ID_AAC))
							.ok_or(FfmpegError::NoEncoder)
							.context("failed to find aac encoder")?;

						(
							codec,
							Dictionary::builder()
								.set(
									"profile",
									match object_type {
										aac::AudioObjectType::AacLowComplexity => "aac_low",
										aac::AudioObjectType::AacMain => "aac_main",
										aac::AudioObjectType::Unknown(profile) => {
											anyhow::bail!("invalid aac profile: {profile}");
										}
									},
								)
								.build(),
						)
					}
					AudioCodec::Opus => {
						let codec = ffmpeg::codec::EncoderCodec::by_name("libopus")
							.or_else(|| ffmpeg::codec::EncoderCodec::new(AVCodecID::AV_CODEC_ID_OPUS))
							.ok_or(FfmpegError::NoEncoder)
							.context("failed to find opus encoder")?;

						(codec, Dictionary::new())
					}
				};

				let sender = outputs
					.remove(&Rendition::from(audio_config.rendition()))
					.ok_or_else(|| anyhow::anyhow!("missing audio output"))?;
				this.setup_audio_encoder(sender, &audio_config, &audio_stream, encoder_codec, encoder_options)?;
			}
		}

		this.input = Some(ictx);

		if !outputs.is_empty() {
			anyhow::bail!("missing outputs: {:?}", outputs.keys());
		}

		Ok(this)
	}

	pub fn run(&mut self) -> anyhow::Result<()> {
		let mut packets = self.input.as_mut().unwrap().packets();

		while let Some(mut packet) = packets.receive_packet().context("failed to receive packet")? {
			let stream_idx = packet.stream_index();
			packet.set_stream_index(0);

			if stream_idx == self.video_stream_index {
				handle_video_packet(
					packet,
					&mut self.video_copies,
					&mut self.video_decoder,
					&mut self.video_encoders,
					&mut self.scalars,
					&mut self.limiters,
				)
				.context("video")?;
			} else if stream_idx == self.audio_stream_index {
				handle_audio_packet(
					packet,
					&mut self.audio_copies,
					self.audio_decoder.as_mut(),
					&mut self.audio_encoders,
				)
				.context("audio")?;
			}
		}

		handle_audio_eof(&mut self.audio_copies, self.audio_decoder.as_mut(), &mut self.audio_encoders)
			.context("audio eof")?;

		handle_video_eof(
			&mut self.video_copies,
			&mut self.video_decoder,
			&mut self.video_encoders,
			&mut self.scalars,
			&mut self.limiters,
		)
		.context("video eof")?;

		Ok(())
	}

	fn setup_video_encoder(
		&mut self,
		sender: mpsc::Sender<Vec<u8>>,
		video_config: &VideoConfig,
		video_stream: &Stream<'_>,
		encoder_codec: EncoderCodec,
		encoder_options: Dictionary,
	) -> anyhow::Result<()> {
		let output = ffmpeg::io::Output::new(
			sender.into_compat(),
			OutputOptions {
				format_name: Some("mp4"),
				..Default::default()
			},
		)
		.context("failed to create output")?;

		self.limiters
			.push(Limiter::new(video_config.fps, self.video_decoder.time_base()));

		self.scalars.push(Scalar::new(
			self.scalars
				.last()
				.map(|s| s.width())
				.unwrap_or_else(|| self.video_decoder.width()),
			self.scalars
				.last()
				.map(|s| s.height())
				.unwrap_or_else(|| self.video_decoder.height()),
			self.scalars
				.last()
				.map(|s| s.pixel_format())
				.unwrap_or_else(|| self.video_decoder.pixel_format()),
			video_config.width,
			video_config.height,
			self.video_decoder.pixel_format(),
		)?);

		self.video_encoders.push(MuxerEncoder::new(
			encoder_codec,
			output,
			self.video_decoder.time_base(),
			AVRational {
				num: 1,
				den: 1000 * video_stream.r_frame_rate().num,
			},
			VideoEncoderSettings::builder(
				video_config.width,
				video_config.height,
				video_config.fps,
				self.video_decoder.pixel_format(),
			)
			.bitrate(video_config.bitrate)
			.rc_max_rate(video_config.bitrate)
			.rc_buffer_size(video_config.bitrate as i32 * 2)
			.gop_size(video_config.fps * 2)
			.max_b_frames(0)
			.thread_count(0)
			.codec_specific_options(encoder_options)
			.build(),
			MuxerSettings::builder()
				.interleave(true)
				.muxer_options(muxer_options())
				.build(),
		)?);

		Ok(())
	}

	fn setup_audio_encoder(
		&mut self,
		sender: mpsc::Sender<Vec<u8>>,
		audio_config: &AudioConfig,
		audio_stream: &Stream<'_>,
		encoder_codec: EncoderCodec,
		encoder_options: Dictionary,
	) -> anyhow::Result<()> {
		let output = ffmpeg::io::Output::new(
			sender.into_compat(),
			OutputOptions {
				format_name: Some("mp4"),
				..Default::default()
			},
		)
		.context("failed to create output")?;

		self.audio_encoders.push(MuxerEncoder::new(
			encoder_codec,
			output,
			self.audio_decoder.as_ref().unwrap().time_base(),
			AVRational {
				num: 1,
				den: audio_stream.r_frame_rate().num,
			},
			AudioEncoderSettings::builder(
				audio_config.sample_rate,
				self.audio_decoder.as_ref().unwrap().channel_layout(),
				self.audio_decoder.as_ref().unwrap().channels(),
				self.audio_decoder.as_ref().unwrap().sample_format(),
			)
			.bitrate(audio_config.bitrate)
			.rc_max_rate(audio_config.bitrate)
			.rc_buffer_size(audio_config.bitrate as i32 * 2)
			.thread_count(1)
			.codec_specific_options(encoder_options)
			.build(),
			MuxerSettings::builder()
				.interleave(true)
				.muxer_options(muxer_options())
				.build(),
		)?);

		Ok(())
	}
}
