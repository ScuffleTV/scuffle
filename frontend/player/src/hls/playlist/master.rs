use std::collections::HashMap;

use serde::Serialize;

use super::utils::Tag;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MasterPlaylist {
    pub streams: Vec<Stream>,
    pub groups: HashMap<String, Vec<Media>>,
    pub scuf_groups: HashMap<String, ScufGroup>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ScufGroup {
    pub priority: i32,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum MediaType {
    Audio,
    Video,
    Subtitles,
    ClosedCaptions,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Media {
    pub media_type: MediaType,
    pub uri: String,
    pub name: String,
    pub group_id: String,
    pub codecs: String,
    pub bandwidth: u32,
    pub autoselect: bool,
    pub default: bool,
    pub forced: bool,
    pub resolution: Option<(u32, u32)>,
    pub frame_rate: Option<f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Stream {
    pub uri: String,
    pub bandwidth: u32,
    pub name: String,
    pub group: String,
    pub codecs: String,
    pub resolution: Option<(u32, u32)>,
    pub frame_rate: Option<f64>,
    pub audio: Option<String>,
    pub video: Option<String>,
}

impl MasterPlaylist {
    pub fn from_tags(tags: Vec<Tag>) -> Result<Self, String> {
        let streams = tags
            .iter()
            .filter_map(|t| match t {
                Tag::ExtXStreamInf(attributes, uri) => Some(Stream {
                    uri: uri.clone(),
                    name: attributes.get("NAME").map(|s| s.to_string())?,
                    group: attributes.get("GROUP").map(|s| s.to_string())?,
                    audio: attributes.get("AUDIO").map(|s| s.to_string()),
                    video: attributes.get("VIDEO").map(|s| s.to_string()),
                    bandwidth: attributes
                        .get("BANDWIDTH")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0),
                    codecs: attributes.get("CODECS").map(|s| s.to_string())?,
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

                    let group_id = attributes
                        .get("GROUP-ID")
                        .map(|s| s.to_string())
                        .unwrap_or_default();

                    let bandwidth = attributes
                        .get("BANDWIDTH")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);

                    let codecs = attributes
                        .get("CODECS")
                        .map(|s| s.to_string())
                        .unwrap_or_default();

                    let resolution = attributes.get("RESOLUTION").and_then(|s| {
                        let mut parts = s.split('x');
                        let width = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                        let height = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                        if width == 0 || height == 0 {
                            None
                        } else {
                            Some((width, height))
                        }
                    });

                    let frame_rate = attributes.get("FRAME-RATE").and_then(|s| s.parse().ok());

                    Some((
                        group_id.clone(),
                        Media {
                            media_type,
                            uri,
                            name,
                            autoselect,
                            default,
                            forced,
                            group_id,
                            bandwidth,
                            codecs,
                            resolution,
                            frame_rate,
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

        let scuf_groups = tags
            .iter()
            .filter_map(|t| match t {
                Tag::ExtXScufGroup(attributes) => {
                    let priority = attributes
                        .get("PRIORITY")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);

                    Some((
                        attributes
                            .get("GROUP")
                            .map(|s| s.to_string())
                            .unwrap_or_default(),
                        ScufGroup { priority },
                    ))
                }
                _ => None,
            })
            .collect();

        Ok(Self {
            streams,
            groups,
            scuf_groups,
        })
    }
}
