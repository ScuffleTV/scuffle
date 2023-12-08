use crate::processor::job::decoder::{Decoder, DecoderBackend};
use crate::processor::job::resize::{ImageResizer, ImageResizerTarget};
use crate::tests::utils::asset_path;

fn resize(asset_name: &str, backend: DecoderBackend) {
	let input_path = asset_path(asset_name);

	let start = std::time::Instant::now();

	let mut decoder = backend
		.build(input_path.as_path(), &Default::default())
		.expect("decoder build error");

	let mut resizer = ImageResizer::new(ImageResizerTarget {
		height: 30,
		width: 30,
		..Default::default()
	});

	while let Some(frame) = decoder.decode().expect("frame decode error") {
		let resized = resizer.resize(frame.as_ref()).expect("resize error");

		assert_eq!(resized.as_ref().image.width(), 30, "width mismatch");
		assert_eq!(resized.as_ref().image.height(), 30, "height mismatch");
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
