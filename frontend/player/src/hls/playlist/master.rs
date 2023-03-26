use std::collections::HashMap;

use serde::Serialize;

use super::utils::Tag;

#[derive(Debug, Clone, Serialize)]
pub struct MasterPlaylist {
    pub streams: Vec<Stream>,
    pub groups: HashMap<String, Vec<Media>>,
}

#[derive(Debug, Clone, Serialize)]
pub enum MediaType {
    Audio,
    Video,
    Subtitles,
    ClosedCaptions,
}

#[derive(Debug, Clone, Serialize)]
pub struct Media {
    pub media_type: MediaType,
    pub uri: String,
    pub name: String,
    pub autoselect: bool,
    pub default: bool,
    pub forced: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Stream {
    pub uri: String,
    pub bandwidth: u32,
    pub average_bandwidth: Option<u32>,
    pub codecs: Option<String>,
    pub resolution: Option<(u32, u32)>,
    pub frame_rate: Option<f64>,
    pub hdcp_level: Option<String>,
    pub audio: Option<String>,
    pub video: Option<String>,
    pub subtitles: Option<String>,
    pub closed_captions: Option<String>,
}

impl MasterPlaylist {
    pub fn from_tags(tags: Vec<Tag>) -> Result<Self, String> {
        let streams = tags
            .iter()
            .filter_map(|t| match t {
                Tag::ExtXStreamInf(attributes, uri) => Some(Stream {
                    uri: uri.clone(),
                    audio: attributes.get("AUDIO").map(|s| s.to_string()),
                    video: attributes.get("VIDEO").map(|s| s.to_string()),
                    subtitles: attributes.get("SUBTITLES").map(|s| s.to_string()),
                    closed_captions: attributes.get("CLOSED-CAPTIONS").map(|s| s.to_string()),
                    bandwidth: attributes
                        .get("BANDWIDTH")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0),
                    average_bandwidth: attributes
                        .get("AVERAGE-BANDWIDTH")
                        .and_then(|s| s.parse().ok()),
                    codecs: attributes.get("CODECS").map(|s| s.to_string()),
                    resolution: attributes.get("RESOLUTION").and_then(|s| {
                        let mut parts = s.split('x');
                        let width = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                        let height = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                        if width == 0 || height == 0 {
                            None
                        } else {
                            Some((width, height))
                        }
                    }),
                    frame_rate: attributes.get("FRAME-RATE").and_then(|s| s.parse().ok()),
                    hdcp_level: attributes.get("HDCP-LEVEL").map(|s| s.to_string()),
                }),
                _ => None,
            })
            .collect();

        let groups = tags
            .iter()
            .filter_map(|t| match t {
                Tag::ExtXMedia(attributes) => {
                    let media_type = match attributes.get("TYPE").map(|s| s.as_str()) {
                        Some("AUDIO") => MediaType::Audio,
                        Some("VIDEO") => MediaType::Video,
                        Some("SUBTITLES") => MediaType::Subtitles,
                        Some("CLOSED-CAPTIONS") => MediaType::ClosedCaptions,
                        _ => return None,
                    };

                    let uri = attributes
                        .get("URI")
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    let name = attributes
                        .get("NAME")
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    let autoselect = attributes
                        .get("AUTOSELECT")
                        .map(|s| s == "YES")
                        .unwrap_or_default();
                    let default = attributes
                        .get("DEFAULT")
                        .map(|s| s == "YES")
                        .unwrap_or_default();
                    let forced = attributes
                        .get("FORCED")
                        .map(|s| s == "YES")
                        .unwrap_or_default();

                    Some((
                        attributes
                            .get("GROUP-ID")
                            .map(|s| s.to_string())
                            .unwrap_or_default(),
                        Media {
                            media_type,
                            uri,
                            name,
                            autoselect,
                            default,
                            forced,
                        },
                    ))
                }
                _ => None,
            })
            .fold(
                HashMap::<String, Vec<Media>>::new(),
                |mut groups, (group_id, media)| {
                    groups.entry(group_id).or_default().push(media);
                    groups
                },
            );

        Ok(Self { streams, groups })
    }
}
