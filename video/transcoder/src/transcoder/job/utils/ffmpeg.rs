use std::{os::unix::process::CommandExt, path::Path};

use common::vec_of_strings;
use mp4::codec::{AudioCodec, VideoCodec};
use pb::scuffle::video::v1::types::{AudioConfig, Rendition as PbRendition, VideoConfig};
use tokio::process::{Child, Command};
use video_common::database::Rendition;

pub fn spawn_ffmpeg(
    gid: u32,
    uid: u32,
    socket_dir: &Path,
    video_output: &[VideoConfig],
    audio_output: &[AudioConfig],
) -> anyhow::Result<Child> {
    let filter_graph_items = video_output
        .iter()
        .filter(|v| v.rendition() != PbRendition::VideoSource)
        .collect::<Vec<_>>();

    let filter_graph = filter_graph_items
        .iter()
        .enumerate()
        .map(|(i, settings)| {
            let previous = if i == 0 {
                "[0:v]".to_string()
            } else {
                format!("[{}_out]", i - 1)
            };

            let rendition = Rendition::from(settings.rendition());

            format!(
                "{}scale={}:{},pad=ceil(iw/2)*2:ceil(ih/2)*2{}",
                previous,
                settings.width,
                settings.height,
                if i == filter_graph_items.len() - 1 {
                    format!("[{rendition}]")
                } else {
                    format!(",split=2[{rendition}][{i}_out]")
                }
            )
        })
        .collect::<Vec<_>>()
        .join(";");

    const MP4_FLAGS: &str = "+frag_keyframe+empty_moov+default_base_moof";

    #[rustfmt::skip]
    let mut args = vec_of_strings![
        "-v", "error",
        "-i", "-",
        "-probesize", "250M",
        "-analyzeduration", "250M",
        "-max_muxing_queue_size", "1024",
    ];

    if !filter_graph.is_empty() {
        args.extend(vec_of_strings!["-filter_complex", filter_graph]);
    }

    for output in video_output {
        let rendition = Rendition::from(output.rendition());

        if output.rendition() == PbRendition::VideoSource {
            #[rustfmt::skip]
            args.extend(vec_of_strings![
                "-map", "0:v",
                "-c:v", "copy",
            ]);
        } else {
            let codec: VideoCodec = match output.codec.parse() {
                Ok(c) => c,
                Err(err) => {
                    anyhow::bail!("invalid video codec: {}", err);
                }
            };

            match codec {
                VideoCodec::Avc { profile, level, .. } => {
                    #[rustfmt::skip]
                    args.extend(vec_of_strings![
                        "-map", format!("[{rendition}]"),
                        "-c:v", "libx264",
                        "-preset", "medium",
                        "-b:v", format!("{}", output.bitrate),
                        "-maxrate", format!("{}", output.bitrate),
                        "-bufsize", format!("{}", output.bitrate * 2),
                        "-profile:v", match profile {
                            66 => "baseline",
                            77 => "main",
                            100 => "high",
                            _ => {
                                anyhow::bail!("invalid avc profile: {}", profile);
                            },
                        },
                        "-level:v", match level {
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
                                anyhow::bail!("invalid avc level: {}", level);
                            },
                        },
                        "-pix_fmt", "yuv420p",
                        "-g", format!("{}", output.fps * 2),
                        "-keyint_min", format!("{}", output.fps * 2),
                        "-sc_threshold", "0",
                        "-r", format!("{}", output.fps),
                        "-crf", "23",
                        "-tune", "zerolatency",
                    ]);
                }
                VideoCodec::Av1 { .. } => {
                    anyhow::bail!("av1 is not supported");
                }
                VideoCodec::Hevc { .. } => {
                    anyhow::bail!("hevc is not supported");
                }
            }
        }

        #[rustfmt::skip]
        args.extend(vec_of_strings![
            "-f", "mp4",
            "-movflags", MP4_FLAGS,
            "-frag_duration", "1",
            format!(
                "unix://{}",
                socket_dir.join(format!("{}.sock", rendition)).display()
            ),
        ]);
    }

    for output in audio_output {
        let rendition = Rendition::from(output.rendition());

        if output.rendition() == PbRendition::AudioSource {
            #[rustfmt::skip]
            args.extend(vec_of_strings![
                "-map", "0:a",
                "-c:a", "copy",
            ]);
        } else {
            let codec: AudioCodec = match output.codec.parse() {
                Ok(c) => c,
                Err(err) => {
                    anyhow::bail!("invalid audio codec: {}", err);
                }
            };

            match codec {
                AudioCodec::Aac { object_type } => {
                    #[rustfmt::skip]
                    args.extend(vec_of_strings![
                        "-map", "0:a",
                        "-c:a", "aac",
                        "-b:a", format!("{}", output.bitrate),
                        "-ar", format!("{}", output.sample_rate),
                        "-ac", format!("{}", output.channels),
                        "-profile:a",
                        match object_type {
                            aac::AudioObjectType::AacLowComplexity => {
                                "aac_low"
                            }
                            aac::AudioObjectType::AacMain => {
                                "aac_main"
                            }
                            aac::AudioObjectType::Unknown(profile) => {
                                anyhow::bail!("invalid aac profile: {}", profile);
                            }
                        },
                    ]);
                }
                AudioCodec::Opus => {
                    #[rustfmt::skip]
                    args.extend(vec_of_strings![
                        "-map", "0:a",
                        "-c:a", "libopus",
                        "-b:a", format!("{}", output.bitrate),
                        "-ar", format!("{}", output.sample_rate),
                        "-ac", format!("{}", output.channels),
                    ]);
                }
            }
        }

        #[rustfmt::skip]
        args.extend(vec_of_strings![
            "-f", "mp4",
            "-movflags", MP4_FLAGS,
            "-frag_duration", "1",
            format!(
                "unix://{}",
                socket_dir.join(format!("{}.sock", rendition)).display()
            ),
        ]);
    }

    let mut child = std::process::Command::new("ffmpeg");

    child
        .args(&args)
        .stdin(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::null())
        .process_group(0)
        .uid(uid)
        .gid(gid)
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap_or_default());

    Ok(match Command::from(child).kill_on_drop(true).spawn() {
        Ok(c) => c,
        Err(err) => {
            anyhow::bail!("failed to spawn ffmpeg: {}", err);
        }
    })
}

pub fn spawn_ffmpeg_screenshot(
    gid: u32,
    uid: u32,
    width: i32,
    height: i32,
) -> anyhow::Result<Child> {
    #[rustfmt::skip]
    let args = vec_of_strings![
        "-v", "error",
        "-i", "-",
        "-threads", "1",
        "-analyzeduration", "32",
        "-probesize", "32",
        "-frames:v", "1",
        "-f", "image2pipe",
        "-c:v", "mjpeg",
        "-vf", format!("scale={}:{}", width, height),
        "-",
    ];

    let mut child = std::process::Command::new("ffmpeg");

    child
        .args(&args)
        .stdin(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::piped())
        .process_group(0)
        .uid(uid)
        .gid(gid)
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap_or_default());

    Ok(match Command::from(child).kill_on_drop(true).spawn() {
        Ok(c) => c,
        Err(err) => {
            anyhow::bail!("failed to spawn ffmpeg: {}", err);
        }
    })
}
