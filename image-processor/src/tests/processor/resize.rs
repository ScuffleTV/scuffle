use std::borrow::Cow;

use crate::processor::job::decoder::{Decoder, DecoderBackend};
use crate::processor::job::resize::{ImageResizer, ImageResizerTarget};
use crate::tests::utils::asset_bytes;

fn resize(asset_name: &str, backend: DecoderBackend) {
	let input_bytes = asset_bytes(asset_name);

	let start = std::time::Instant::now();

	let mut decoder = backend
		.build(&Default::default(), Cow::Owned(input_bytes))
		.expect("decoder build error");

	let mut resizer = ImageResizer::new(ImageResizerTarget {
		height: 30,
		width: 30,
		..Default::default()
	});

	while let Some(frame) = decoder.decode().expect("frame decode error") {
		let resized = resizer.resize(&frame).expect("resize error");

		assert_eq!(resized.image.width(), 30, "width mismatch");
		assert_eq!(resized.image.height(), 30, "height mismatch");
	}

	println!("decode time ({asset_name}): {:?}", start.elapsed());
}

#[test]
fn resize_gif_test() {
	resize("meow.gif", DecoderBackend::Ffmpeg);
}

#[test]
fn resize_webp_test() {
	resize("meow.webp", DecoderBackend::LibWebp);
}

#[test]
fn resize_avif_test() {
	resize("meow.avif", DecoderBackend::LibAvif);
}
