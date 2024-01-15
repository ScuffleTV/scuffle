use std::borrow::Cow;
use std::collections::HashMap;

use rgb::ComponentBytes;
use sha2::Digest;

use crate::processor::error::ProcessorError;
use crate::processor::job::decoder::{Decoder, DecoderBackend};
use crate::processor::job::encoder::{Encoder, EncoderFrontend, EncoderSettings};
use crate::processor::job::resize::{ImageResizer, ImageResizerTarget};
use crate::tests::utils::asset_bytes;

fn encode(asset_name: &str, backend: DecoderBackend, frontend: EncoderFrontend) {
	let input_bytes = asset_bytes(asset_name);

	let start = std::time::Instant::now();

	let mut decoder = backend
		.build(&Default::default(), Cow::Owned(input_bytes))
		.expect("failed to build decoder");

	let info = decoder.info();

	let mut resizer = ImageResizer::new(ImageResizerTarget {
		height: 30,
		width: 30,
		..Default::default()
	});

	let mut frames = Vec::with_capacity(info.frame_count);
	let mut frame_hashes = HashMap::new();
	let mut frame_order = Vec::with_capacity(info.frame_count);
	let mut count = 0;

	while let Some(frame) = decoder.decode().expect("failed to decode") {
		let hash = sha2::Sha256::digest(frame.image.buf().as_bytes());
		if let Some(idx) = frame_hashes.get(&hash) {
			if let Some((last_idx, last_duration)) = frame_order.last_mut() {
				if last_idx == idx {
					*last_duration += frame.duration_ts;
				} else {
					frame_order.push((*idx, frame.duration_ts));
				}
			} else {
				frame_order.push((*idx, frame.duration_ts));
			}
		} else {
			frame_hashes.insert(hash, count);
			frame_order.push((count, frame.duration_ts));

			count += 1;
			frames.push(resizer.resize(&frame).expect("failed to resize"));
		}
	}

	let mut encoder = frontend
		.build(EncoderSettings {
			fast: true,
			loop_count: info.loop_count,
			timescale: info.timescale,
			static_image: false,
		})
		.expect("failed to build encoder");

	for (idx, timing) in frame_order.into_iter() {
		let resized = &mut frames[idx];
		resized.duration_ts = timing;
		encoder.add_frame(resized).expect("failed to add frame");
	}

	let info = encoder.info();
	dbg!(&info);
	let output = encoder.finish().expect("failed to finish");
	let output_path = format!(
		"/tmp/{}x{}.{}",
		info.width,
		info.height,
		match info.frontend {
			EncoderFrontend::Gifski => "gif",
			EncoderFrontend::LibAvif => "avif",
			EncoderFrontend::LibWebp => "webp",
			EncoderFrontend::Png => "png",
		}
	);
	std::fs::write(&output_path, output)
		.map_err(ProcessorError::FileCreate)
		.expect("failed to write output");
	println!("wrote output to {}", output_path);

	println!("encode time ({asset_name}): {:?}", start.elapsed());
}

#[test]
fn encode_test() {
	encode("cat.gif", DecoderBackend::Ffmpeg, EncoderFrontend::LibWebp);
	encode("meow.webp", DecoderBackend::LibWebp, EncoderFrontend::LibAvif);
	encode("meow.avif", DecoderBackend::LibAvif, EncoderFrontend::Gifski);
}
