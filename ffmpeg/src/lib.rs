pub mod codec;
pub mod consts;
pub mod decoder;
pub mod dict;
pub mod encoder;
pub mod error;
pub mod filter_graph;
pub mod frame;
pub mod io;
pub mod limiter;
pub mod packet;
pub mod scalar;
pub mod stream;
pub mod utils;

pub use ffmpeg_sys_next as ffi;

mod smart_object;

// fn main() {
//     unsafe { ffmpeg_sys_next::av_log_set_level(ffmpeg_sys_next::AV_LOG_ERROR)
// };     let data = std::fs::read("input-frag.mp4").unwrap();

//     const X264_COUNT: usize = 10;
//     const NVENC_COUNT: usize = 0;
//     const TOTAL_COUNT: usize = NVENC_COUNT + X264_COUNT;

//     let encoders = std::iter::repeat("h264_nvenc")
//         .take(NVENC_COUNT)
//         .chain(std::iter::repeat("libx264").take(X264_COUNT));

//     std::thread::scope(|s| {
//         let data = &data;
//         for encoder in encoders {
//             s.spawn(move || {
//                 reencode(data, encoder).unwrap();
//             });
//         }
//     })
// }

// fn reencode(
//     data: &[u8],
//     encoder: &str,
// ) -> Result<(), FfmpegError> {
//     let start = Instant::now();

//     let mut ictx = io::Input::new(std::io::Cursor::new(data))?;

//     let video_stream = ictx
//         .streams()
//         .best(AVMediaType::AVMEDIA_TYPE_VIDEO)
//         .ok_or(FfmpegError::NoStream)?;

//     let mut copies = vec![io::Output::new(
//         Vec::new(),
//         OutputOptions::new().format_name("mp4"),
//     )?];

//     let muxer_options = Dictionary::builder()
//         .set(
//             "movflags",
//
// "frag_keyframe+frag_every_frame+empty_moov+delay_moov+default_base_moof",
//         )
//         .build();

//     copies.iter_mut().try_for_each(|copy| {
//         copy.copy_stream(&video_stream)
//             .ok_or(FfmpegError::NoStream)?;
//         copy.write_header_with_options(&mut muxer_options.clone())
//     })?;

//     let thread_count = 1;

//     let video_frame_rate = (video_stream.r_frame_rate().num as f64
//         / video_stream.r_frame_rate().den as f64)
//         .round() as i32;

//     let sizes = vec![
//         (1280, 720, 60.min(video_frame_rate)),
//         (854, 480, 30.min(video_frame_rate)),
//         (640, 360, 30.min(video_frame_rate)),
//     ];

//     let mut decoder = if sizes.is_empty() {
//         None
//     } else {
//         match decoder::Decoder::with_options(
//             &video_stream,
//             DecoderOptions {
//                 thread_count,
//                 ..Default::default()
//             },
//         )? {
//             decoder::Decoder::Video(decoder) => Some(decoder),
//             _ => unreachable!(),
//         }
//     };

//     let (mut resizers, mut encoders, mut limiters) = if let Some(decoder) =
// decoder.as_ref() {         let resizers = sizes
//             .iter()
//             .copied()
//             .enumerate()
//             .map(|(idx, (width, height, _))| {
//                 let (input_width, input_height, _) = idx
//                     .checked_sub(1)
//                     .map(|idx| sizes[idx])
//                     .unwrap_or_else(|| (decoder.width(), decoder.height(),
// 1));

//                 scalar::Scalar::new(
//                     input_width,
//                     input_height,
//                     decoder.pixel_format(),
//                     width,
//                     height,
//                     decoder.pixel_format(),
//                 )
//             })
//             .collect::<Result<Vec<_>, _>>()?;

//         let encoders = sizes
//             .iter()
//             .copied()
//             .map(|(width, height, fps)| {
//                 let enc = encoder::Encoder::with_output(
//
// EncoderCodec::by_name(encoder).ok_or(FfmpegError::NoEncoder)?,
// io::Output::new(Vec::new(), OutputOptions::new().format_name("mp4"))?,
//                     decoder.time_base(),
//                     AVRational {
//                         num: 1,
//                         den: 1000 * video_stream.r_frame_rate().num,
//                     },
//                     &mut Dictionary::builder()
//                         .set("threads", "1")
//                         .set("preset", "medium")
//                         .set("tune", "zerolatency")
//                         .set("profile", "high")
//                         .set("level", "4.2")
//                         .build(),
//                     encoder::EncoderSettings::Video {
//                         width,
//                         height,
//                         frame_rate: fps,
//                         gop_size: fps * 2,
//                         qmax: 0,
//                         qmin: 0,
//                         pixel_format: decoder.pixel_format(),
//                         thread_count,
//                     },
//                     false,
//                     muxer_options.clone(),
//                 )?;

//                 Ok(enc)
//             })
//             .collect::<Result<Vec<_>, _>>()?;

//         let limiter = sizes
//             .iter()
//             .copied()
//             .map(|(_, _, fps)| limiter::FrameRateLimiter::new(fps,
// decoder.time_base()))             .collect::<Vec<_>>();

//         (resizers, encoders, limiter)
//     } else {
//         (vec![], vec![], vec![])
//     };

//     let in_video_stream_idx = video_stream.index();

//     let mut frame_timings = Vec::with_capacity(100);
//     let frame_time =
//         1000.0 / (video_stream.r_frame_rate().num as f64 /
// video_stream.r_frame_rate().den as f64);

//     println!("ready: {}ms", start.elapsed().as_millis());

//     let mut frame_start = Instant::now();
//     let mut handle_frame = |mut frame: frame::VideoFrame| -> Result<(),
// FfmpegError> {         frame.
// set_pict_type(AVPictureType::AV_PICTURE_TYPE_NONE);

//         let mut frames = vec![];
//         let frame_ref = &frame.0;
//         for (resizer, limiter) in
// resizers.iter_mut().zip(limiters.iter_mut()) {             if
// !limiter.limit(&frame) {                 break;
//             }

// frames.push(resizer.proces(*frames.last().unwrap_or(&frame_ref))?);         }

//         encoders
//             .iter_mut()
//             .zip(frames.iter())
//             .try_for_each(|(encoder, frame)| encoder.send_frame(frame))?;

//         let elapsed = frame_start.elapsed().as_secs_f64() * 1000.0;
//         frame_timings.push(elapsed);
//         if frame_timings.len() > 100 {
//             let avg = frame_timings.iter().sum::<f64>() / frame_timings.len()
// as f64;             if avg > frame_time {
//                 println!("dropping frames: {avg}ms");
//             }

//             frame_timings.clear();
//         }

//         frame_start = Instant::now();

//         Ok(())
//     };

//     let start = Instant::now();

//     let mut i = 0;

//     let mut packets = ictx.packets()?;
//     while let Some(packet) = packets.receive_packet()? {
//         if packet.stream_index() != in_video_stream_idx {
//             continue;
//         }

//         i += 1;
//         if i > 600 {
//             // break;
//         }

//         copies
//             .iter_mut()
//             .try_for_each(|copy| copy.write_packet(&packet))?;

//         if let Some(decoder) = decoder.as_mut() {
//             decoder.send_packet(&packet)?;

//             while let Some(frame) = decoder.receive_frame()? {
//                 handle_frame(frame)?;
//             }
//         }
//     }

//     copies
//         .iter_mut()
//         .try_for_each(|copy| copy.write_trailer())?;

//     if let Some(decoder) = decoder.as_mut() {
//         decoder.send_eof()?;
//         while let Some(frame) = decoder.receive_frame()? {
//             handle_frame(frame)?;
//         }

//         encoders
//             .iter_mut()
//             .try_for_each(|encoder| encoder.send_eof())?;
//     }

//     println!("done: {}ms", start.elapsed().as_millis());

//     encoders.into_iter().enumerate().for_each(|(idx, encoder)| {
//         let data = encoder.into_inner().into_inner();
//         std::fs::write(format!("output-e{idx}.mp4"), data).unwrap();
//     });

//     copies.into_iter().enumerate().for_each(|(idx, copy)| {
//         let data = copy.into_inner();
//         std::fs::write(format!("output-c{idx}.mp4"), data).unwrap();
//     });

//     Ok(())
// }
