use std::ffi::CString;

use ffmpeg_next::{codec, encoder, media, Rational};
use ffmpeg_next_io::OutputOptions;

fn main() {
	let input_file_path = std::env::args().nth(1).expect("missing input file");
	let output_file_path = std::env::args().nth(2).expect("missing output file");

	let input_file = std::fs::File::open(input_file_path).expect("failed to open input file");
	let output_file = std::fs::File::create(&output_file_path).expect("failed to open output file");

	let format_ffi = unsafe {
		ffmpeg_next::ffi::av_guess_format(
			std::ptr::null(),
			CString::from_vec_unchecked(output_file_path.as_bytes().to_vec()).as_ptr(),
			std::ptr::null(),
		)
	};
	if format_ffi.is_null() {
		panic!("failed to guess output format from path");
	}

	ffmpeg_next::init().unwrap();

	let mut ictx = ffmpeg_next_io::Input::seekable(input_file).unwrap();
	let mut octx = ffmpeg_next_io::Output::seekable(
		output_file,
		OutputOptions {
			format_ffi,
			..Default::default()
		},
	)
	.unwrap();

	let mut stream_mapping = vec![0; ictx.nb_streams() as _];
	let mut ist_time_bases = vec![Rational(0, 1); ictx.nb_streams() as _];
	let mut ost_index = 0;
	for (ist_index, ist) in ictx.streams().enumerate() {
		let ist_medium = ist.parameters().medium();
		if ist_medium != media::Type::Audio && ist_medium != media::Type::Video && ist_medium != media::Type::Subtitle {
			stream_mapping[ist_index] = -1;
			continue;
		}
		stream_mapping[ist_index] = ost_index;
		ist_time_bases[ist_index] = ist.time_base();
		ost_index += 1;
		let mut ost = octx.add_stream(encoder::find(codec::Id::None)).unwrap();
		ost.set_parameters(ist.parameters());
		// We need to set codec_tag to 0 lest we run into incompatible codec tag
		// issues when muxing into a different container format. Unfortunately
		// there's no high level API to do this (yet).
		unsafe {
			(*ost.parameters().as_mut_ptr()).codec_tag = 0;
		}
	}

	octx.set_metadata(ictx.metadata().to_owned());
	octx.write_header().unwrap();

	for (stream, mut packet) in ictx.packets() {
		let ist_index = stream.index();
		let ost_index = stream_mapping[ist_index];
		if ost_index < 0 {
			continue;
		}
		let ost = octx.stream(ost_index as _).unwrap();
		packet.rescale_ts(ist_time_bases[ist_index], ost.time_base());
		packet.set_position(-1);
		packet.set_stream(ost_index as _);
		packet.write_interleaved(&mut octx).unwrap();
	}

	octx.write_trailer().unwrap();
}