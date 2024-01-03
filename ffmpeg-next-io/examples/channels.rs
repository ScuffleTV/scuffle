use ffmpeg_next::{codec, encoder, media, Rational};
use ffmpeg_next_io::{ChannelCompatRecv, ChannelCompatSend, OutputOptions};

const DATA: &[u8] = include_bytes!("../assets/video.mp4");

fn main() {
	ffmpeg_next::init().unwrap();

	let (input_tx, input_rx) = std::sync::mpsc::channel::<Vec<u8>>();
	let (output_tx, output_rx) = std::sync::mpsc::channel::<Vec<u8>>();

	let handle = std::thread::spawn(move || {
		let mut ictx = ffmpeg_next_io::Input::new(input_rx.into_compat()).unwrap();
		let mut octx = ffmpeg_next_io::Output::new(
			output_tx.into_compat(),
			OutputOptions {
				format_name: Some("flv"),
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
	});

	// You can stream data from any source into the channel.
	for chunk in DATA.chunks(1024) {
		println!("Sending chunk of size {}.", chunk.len());
		input_tx.send(chunk.to_vec()).unwrap();
	}

	// Drop the input channel to signal that we're done writing.
	drop(input_tx);

	let mut total = 0;
	// You can get the data back via a channel.
	for packet in output_rx {
		println!("Got packet of size {}.", packet.len());
		total += packet.len();
	}

	// Something to keep in mind is that here the channels are unbounded, so
	// you need to be careful not to run into a deadlock. For example, if
	// ffmpeg is blocked on writing to the output channel, it will block
	// the input channel as well. So you should always read from the output
	// channel first or read both concurrently (e.g. with select!)

	println!("Total size: {}.", total);

	handle.join().unwrap();
}
