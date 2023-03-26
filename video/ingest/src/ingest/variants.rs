use serde_json::json;
use transmuxer::{AudioCodec, AudioSettings, VideoCodec, VideoSettings};
use uuid::Uuid;

use crate::pb::scuffle::types::{stream_variant, StreamVariant};

pub fn generate_variants(
    video_settings: &VideoSettings,
    audio_settings: &AudioSettings,
    transcode: bool,
) -> Vec<StreamVariant> {
    let mut variants = Vec::new();

    let audio_settings = stream_variant::AudioSettings {
        channels: audio_settings.channels as u32,
        bitrate: audio_settings.bitrate,
        sample_rate: audio_settings.sample_rate,
        codec: AudioCodec::Opus.to_string(),
    };

    variants.push(StreamVariant {
        id: Uuid::new_v4().to_string(),
        name: "source".to_string(),
        video_settings: Some(stream_variant::VideoSettings {
            bitrate: video_settings.bitrate,
            codec: video_settings.codec.to_string(),
            framerate: video_settings.framerate as u32,
            height: video_settings.height,
            width: video_settings.width,
        }),
        audio_settings: Some(audio_settings.clone()),
        metadata: json!({}).to_string(),
    });

    variants.push(StreamVariant {
        id: Uuid::new_v4().to_string(),
        name: "audio".to_string(),
        video_settings: None,
        audio_settings: Some(audio_settings.clone()),
        metadata: json!({}).to_string(),
    });

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

            variants.push(StreamVariant {
                id: Uuid::new_v4().to_string(),
                name: format!("{}p", res.side),
                video_settings: Some(stream_variant::VideoSettings {
                    width,
                    height,
                    bitrate: res.bitrate,
                    framerate: res.framerate,
                    codec: VideoCodec::Avc {
                        profile: 100, // High
                        level: 51,    // 5.1
                        constraint_set: 0,
                    }
                    .to_string(),
                }),
                audio_settings: Some(audio_settings.clone()),
                metadata: json!({}).to_string(),
            });
        }
    }

    variants
}
