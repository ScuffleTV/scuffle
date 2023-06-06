use aac::AudioObjectType;
use mp4::codec::{AudioCodec, VideoCodec};
use transmuxer::{AudioSettings, VideoSettings};
use uuid::Uuid;

use crate::pb::scuffle::types::{stream_state, StreamState};

pub fn generate_variants(
    video_settings: &VideoSettings,
    _audio_settings: &AudioSettings,
    transcode: bool,
) -> StreamState {
    let mut stream_state = StreamState::default();

    let mut audio_tracks = vec![];

    if transcode {
        let id = Uuid::new_v4().to_string();

        stream_state.transcodes.push(stream_state::Transcode {
            id: id.clone(),
            settings: Some(stream_state::transcode::Settings::Audio(
                stream_state::transcode::AudioSettings {
                    channels: 2,
                    sample_rate: 48000,
                },
            )),
            bitrate: 96 * 1024,
            codec: AudioCodec::Opus.to_string(),
            copy: false,
        });

        stream_state.groups.push(stream_state::Group {
            name: "opus".to_string(),
            priority: 1,
        });

        audio_tracks.push((id, "opus"));
    };

    {
        let id = Uuid::new_v4().to_string();

        stream_state.transcodes.push(stream_state::Transcode {
            id: id.clone(),
            settings: Some(stream_state::transcode::Settings::Audio(
                stream_state::transcode::AudioSettings {
                    channels: 2,
                    sample_rate: 48000,
                },
            )),
            bitrate: 128 * 1024,
            codec: AudioCodec::Aac {
                object_type: AudioObjectType::AacLowComplexity,
            }
            .to_string(),
            copy: false,
        });

        stream_state.groups.push(stream_state::Group {
            name: "aac".to_string(),
            priority: stream_state.groups.len() as i32 + 1,
        });

        audio_tracks.push((id, "aac"));
    };

    stream_state.variants.extend(
        audio_tracks
            .iter()
            .map(|(id, group)| stream_state::Variant {
                name: "audio-only".to_string(),
                group: group.to_string(),
                transcode_ids: vec![id.clone()],
            }),
    );

    {
        let id = Uuid::new_v4().to_string();

        stream_state.transcodes.push(stream_state::Transcode {
            id: id.clone(),
            settings: Some(stream_state::transcode::Settings::Video(
                stream_state::transcode::VideoSettings {
                    framerate: video_settings.framerate as u32,
                    height: video_settings.height,
                    width: video_settings.width,
                },
            )),
            bitrate: video_settings.bitrate,
            codec: video_settings.codec.to_string(),
            copy: true,
        });

        stream_state
            .variants
            .extend(
                audio_tracks
                    .iter()
                    .map(|(track_id, group)| stream_state::Variant {
                        name: "source".to_string(),
                        group: group.to_string(),
                        transcode_ids: vec![id.clone(), track_id.clone()],
                    }),
            );
    }

    if transcode {
        let aspect_ratio = video_settings.width as f64 / video_settings.height as f64;

        struct Resolution {
            side: u32,
            framerate: u32,
            bitrate: u32,
        }

        let resolutions = [
            Resolution {
                bitrate: 4000 * 1024,
                framerate: video_settings.framerate.min(60.0) as u32,
                side: 720,
            },
            Resolution {
                bitrate: 2000 * 1024,
                framerate: video_settings.framerate.min(30.0) as u32,
                side: 480,
            },
            Resolution {
                bitrate: 1000 * 1024,
                framerate: video_settings.framerate.min(30.0) as u32,
                side: 360,
            },
        ];

        for res in resolutions {
            // This prevents us from upscaling the video
            // We only want to downscale the video
            let (width, height) = if aspect_ratio > 1.0 && video_settings.height > res.side {
                ((res.side as f64 * aspect_ratio).round() as u32, res.side)
            } else if aspect_ratio < 1.0 && video_settings.width > res.side {
                (res.side, (res.side as f64 / aspect_ratio).round() as u32)
            } else {
                continue;
            };

            // We dont want to transcode video with resolutions less than 100px on either side
            // We also do not want to transcode anything more expensive than 720p on a 16:9 aspect ratio (720 * 1280)
            // This prevents us from transcoding a "720p" with an aspect ratio of 4:1 (720 * 2880) which is extremely expensive.
            // Just some insight, 2880 / 1280 = 2.25, so this video is 2.25 times more expensive than a normal 720p video.
            // 1080 * 1920 = 2073600
            // 720 * 2880 = 2073600
            // So a 720p video with an aspect ratio of 4:1 is just as expensive as a 1080p video with a 16:9 aspect ratio.
            if width < 100 || height < 100 || width * height > 720 * 1280 {
                continue;
            }

            let id = Uuid::new_v4().to_string();

            stream_state.transcodes.push(stream_state::Transcode {
                id: id.clone(),
                bitrate: res.bitrate,
                codec: VideoCodec::Avc {
                    profile: 100, // High
                    level: 51,    // 5.1
                    constraint_set: 0,
                }
                .to_string(),
                copy: false,
                settings: Some(stream_state::transcode::Settings::Video(
                    stream_state::transcode::VideoSettings {
                        framerate: res.framerate,
                        height,
                        width,
                    },
                )),
            });

            stream_state
                .variants
                .extend(
                    audio_tracks
                        .iter()
                        .map(|(track_id, group)| stream_state::Variant {
                            name: format!("{}p", res.side),
                            group: group.to_string(),
                            transcode_ids: vec![id.clone(), track_id.clone()],
                        }),
                );
        }
    }

    stream_state
}
