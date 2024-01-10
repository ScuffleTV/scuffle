use std::borrow::Cow;

use imgref::ImgVec;

use crate::processor::job::decoder::{Decoder, DecoderBackend, DecoderInfo, LoopCount};
use crate::processor::job::frame::Frame;
use crate::tests::utils::asset_bytes;

fn decode(asset_name: &str, backend: DecoderBackend, expected_info: DecoderInfo, expected_frames: Vec<Frame>) {
	let asset_bytes = asset_bytes(asset_name);

	let start = std::time::Instant::now();

	let mut decoder = backend
		.build(&Default::default(), Cow::Owned(asset_bytes))
		.expect("decoder build error");

	let info = decoder.info();

	assert_eq!(info.frame_count, expected_info.frame_count, "frame count mismatch");
	assert_eq!(info.width, expected_info.width, "width mismatch");
	assert_eq!(info.height, expected_info.height, "height mismatch");
	assert_eq!(info.loop_count, expected_info.loop_count, "loop count mismatch");
	assert_eq!(info.timescale, expected_info.timescale, "timescale mismatch");

	let mut idx = 0;
	while let Some(frame) = decoder.decode().expect("frame decode error") {
		let expected = expected_frames.get(idx).expect("frame count mismatch");
		assert_eq!(frame.duration_ts, expected.duration_ts, "frame duration_ts mismatch: {idx}",);
		assert_eq!(frame.image.height(), expected.image.height(), "frame height mismatch: {idx}",);
		assert_eq!(frame.image.width(), expected.image.width(), "frame width mismatch: {idx}",);
		idx += 1;
	}

	assert_eq!(idx, expected_frames.len(), "frame count mismatch");

	println!("decode time ({asset_name}): {:?}", start.elapsed());
}

#[test]
fn decode_ffmpeg_gif_test() {
	let expected_info = DecoderInfo {
		timescale: 100,
		frame_count: 93,
		loop_count: LoopCount::Infinite,
		height: 128,
		width: 128,
	};

	let expected_frames = (0..93)
		.map(|_| Frame {
			duration_ts: 4,
			image: ImgVec::new(vec![], 128, 128),
		})
		.collect();

	decode("meow.gif", DecoderBackend::Ffmpeg, expected_info, expected_frames);
}

#[test]
fn decode_libwebp_webp_test() {
	let expected_info = DecoderInfo {
		timescale: 1000,
		height: 128,
		width: 128,
		frame_count: 93,
		loop_count: LoopCount::Infinite,
	};

	let expected_frames = (0..93)
		.map(|_| Frame {
			duration_ts: 40,
			image: ImgVec::new(vec![], 128, 128),
		})
		.collect();

	decode("meow.webp", DecoderBackend::LibWebp, expected_info, expected_frames);
}

#[test]
fn decode_libavif_avif_test() {
	let expected_info = DecoderInfo {
		height: 128,
		width: 128,
		frame_count: 93,
		loop_count: LoopCount::Infinite,
		timescale: 100,
	};

	let expected_frames = (0..93)
		.map(|_| Frame {
			image: ImgVec::new(vec![], 128, 128),
			duration_ts: 4,
		})
		.collect();

	decode("meow.avif", DecoderBackend::LibAvif, expected_info, expected_frames);
}
