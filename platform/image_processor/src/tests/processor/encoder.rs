use crate::processor::error::ProcessorError;
use crate::processor::job::decoder::{Decoder, DecoderBackend};
use crate::processor::job::encoder::{Encoder, EncoderFrontend, EncoderSettings};
use crate::processor::job::frame_deduplicator;
use crate::processor::job::resize::{ImageResizer, ImageResizerTarget};
use crate::tests::utils::asset_path;

fn encode(asset_name: &str, backend: DecoderBackend, frontend: EncoderFrontend) {
	let input_path = asset_path(asset_name);

	let start = std::time::Instant::now();

	let mut decoder = backend
		.build(&input_path, &Default::default())
		.expect("failed to build decoder");

	let info = decoder.info();

	let mut resizers = vec![(
		ImageResizer::new(ImageResizerTarget {
			height: 30,
			width: 30,
			..Default::default()
		}),
		vec![
			frontend
				.build(EncoderSettings {
					fast: true,
					loop_count: info.loop_count,
					timescale: info.timescale,
					static_image: false,
				})
				.expect("failed to build encoder"),
		],
	)];

	let mut deduplicator = frame_deduplicator::FrameDeduplicator::new();

	loop {
		let frame = match decoder.decode().expect("failed to decode") {
			Some(frame) => match deduplicator.deduplicate(frame.as_ref()) {
				Some(frame) => frame,
				None => continue,
			},
			None => match deduplicator.flush() {
				Some(frame) => frame,
				None => break,
			},
		};

		for (resizer, encoders) in resizers.iter_mut() {
			let frame = resizer.resize(frame.as_ref()).expect("failed to resize");
			for encoder in encoders.iter_mut() {
				encoder.add_frame(frame.as_ref()).expect("failed to add frame");
			}
		}
	}

	for (_, encoders) in resizers.into_iter() {
		for encoder in encoders.into_iter() {
			let info = encoder.info();
			dbg!(&info);
			let output = encoder.finish().expect("failed to finish");
			let output_path = format!("/tmp/{}x{}.{}", info.width, info.height, info.frontend.extension());
			std::fs::write(&output_path, output)
				.map_err(ProcessorError::FileCreate)
				.expect("failed to write output");
			println!("wrote output to {}", output_path);
		}
	}

	println!("encode time ({asset_name}): {:?}", start.elapsed());
}

#[test]
fn encode_test() {
	encode("meow.gif", DecoderBackend::Ffmpeg, EncoderFrontend::Gifski);
	encode("meow.gif", DecoderBackend::Ffmpeg, EncoderFrontend::LibWebp);
	encode("meow.gif", DecoderBackend::Ffmpeg, EncoderFrontend::LibAvif);
	encode("meow.webp", DecoderBackend::LibWebp, EncoderFrontend::Gifski);
	encode("meow.webp", DecoderBackend::LibWebp, EncoderFrontend::LibWebp);
	encode("meow.webp", DecoderBackend::LibWebp, EncoderFrontend::LibAvif);
	encode("meow.avif", DecoderBackend::LibAvif, EncoderFrontend::Gifski);
	encode("meow.avif", DecoderBackend::LibAvif, EncoderFrontend::LibWebp);
	encode("meow.avif", DecoderBackend::LibAvif, EncoderFrontend::LibAvif);
}
