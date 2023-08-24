use serde::Serialize;

use super::utils::Tag;

#[derive(Debug, Clone, Serialize)]
pub struct MediaPlaylist {
    pub version: u8,
    pub target_duration: u32,
    pub part_target_duration: Option<f64>,
    pub media_sequence: u32,
    pub discontinuity_sequence: u32,
    pub end_list: bool,
    pub server_control: Option<ServerControl>,
    pub segments: Vec<Segment>,
    pub preload_hint: Vec<PreloadHint>,
    pub rendition_reports: Vec<RenditionReport>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RenditionReport {
    pub uri: String,
    pub last_msn: u32,
    pub last_part: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct PreloadHint {
    pub hint_type: String,
    pub uri: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServerControl {
    pub can_skip_until: f64,
    pub hold_back: f64,
    pub part_hold_back: f64,
    pub can_block_reload: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Segment {
    pub discontinuity: bool,
    pub map: Option<String>,
    pub sn: u32,
    pub duration: f64,
    pub url: String,
    pub program_date_time: Option<String>,
    pub gap: bool,
    pub parts: Vec<Part>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Part {
    pub duration: f64,
    pub uri: String,
    pub independent: bool,
}

impl MediaPlaylist {
    pub fn from_tags(tags: Vec<Tag>) -> Result<Self, String> {
        let version = tags
            .iter()
            .find_map(|t| match t {
                Tag::ExtXVersion(v) => Some(*v),
                _ => None,
            })
            .ok_or("no #EXT-X-VERSION tag found")?;

        let target_duration = tags
            .iter()
            .find_map(|t| match t {
                Tag::ExtXTargetDuration(d) => Some(*d),
                _ => None,
            })
            .ok_or("no #EXT-X-TARGETDURATION tag found")?;

        let media_sequence = tags
            .iter()
            .find_map(|t| match t {
                Tag::ExtXMediaSequence(s) => Some(*s),
                _ => None,
            })
            .unwrap_or_default();

        let discontinuity_sequence = tags
            .iter()
            .find_map(|t| match t {
                Tag::ExtXDiscontinuitySequence(s) => Some(*s),
                _ => None,
            })
            .unwrap_or_default();

        let end_list = tags.iter().any(|t| t == &Tag::ExtXEndList);

        let server_control = tags.iter().find_map(|t| match t {
            Tag::ExtXServerControl(attributes) => {
                let can_skip_until = attributes
                    .get("CAN-SKIP-UNTIL")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_default();
                let hold_back = attributes
                    .get("HOLD-BACK")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_default();
                let part_hold_back = attributes
                    .get("PART-HOLD-BACK")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_default();
                let can_block_reload = attributes
                    .get("CAN-BLOCK-RELOAD")
                    .and_then(|v| match v.as_str() {
                        "YES" => Some(true),
                        "NO" => Some(false),
                        _ => v.parse().ok(),
                    })
                    .unwrap_or_default();

                Some(ServerControl {
                    can_skip_until,
                    hold_back,
                    part_hold_back,
                    can_block_reload,
                })
            }
            _ => None,
        });

        let mut map = None;
        let mut sn = media_sequence;
        let mut segments = Vec::new();
        let mut current_segment = None;
        let mut program_date_time = None;
        let mut discontinuity = false;
        let mut part_target_duration = None;

        for tag in tags.iter() {
            match tag {
                Tag::ExtXProgramDateTime(d) => {
                    program_date_time = Some(d);
                }
                Tag::ExtXMap(attributes) => {
                    let uri = attributes.get("URI").ok_or("no URI attribute found")?;
                    map = Some(uri.clone());
                }
                Tag::ExtInf(duration, url) => {
                    let mut current_segment = current_segment.take().unwrap_or_else(|| Segment {
                        discontinuity,
                        map: map.take(),
                        sn,
                        duration: 0.0,
                        url: "".to_string(),
                        program_date_time: program_date_time.cloned(),
                        gap: false,
                        parts: Vec::new(),
                    });

                    current_segment.duration = *duration;
                    current_segment.url = url.clone();

                    discontinuity = false;
                    sn += 1;

                    segments.push(current_segment);
                }
                Tag::ExtXDiscontinuity => {
                    discontinuity = true;
                }
                Tag::ExtXPart(attributes) => {
                    let duration = attributes
                        .get("DURATION")
                        .ok_or("no DURATION attribute found")?;
                    let duration = duration
                        .parse()
                        .map_err(|_| "DURATION attribute is not a number")?;
                    let uri = attributes.get("URI").ok_or("no URI attribute found")?;
                    let independent = attributes
                        .get("INDEPENDENT")
                        .and_then(|v| match v.as_str() {
                            "YES" => Some(true),
                            "NO" => Some(false),
                            _ => v.parse().ok(),
                        })
                        .unwrap_or_default();

                    let part = Part {
                        duration,
                        uri: uri.clone(),
                        independent,
                    };

                    let current_segment = current_segment.get_or_insert_with(|| Segment {
                        discontinuity,
                        map: map.clone(),
                        sn,
                        duration: 0.0,
                        url: "".to_string(),
                        program_date_time: program_date_time.cloned(),
                        gap: false,
                        parts: Vec::new(),
                    });

                    current_segment.parts.push(part);
                }
                Tag::ExtXPartInf(attributes) => {
                    let duration = attributes
                        .get("PART-TARGET")
                        .ok_or("no DURATION attribute found")?;
                    let duration = duration
                        .parse()
                        .map_err(|_| "DURATION attribute is not a number")?;

                    part_target_duration = Some(duration);
                }
                _ => {}
            }
        }

        if let Some(segment) = current_segment {
            segments.push(segment);
        }

        let preload_hint = tags
            .iter()
            .filter_map(|t| match t {
                Tag::ExtXPreloadHint(hint) => {
                    let Some(hint_type) = hint.get("TYPE") else {
                        return Some(Err("no TYPE attribute found"));
                    };
                    let Some(uri) = hint.get("URI") else {
                        return Some(Err("no URI attribute found"));
                    };

                    Some(Ok(PreloadHint {
                        hint_type: hint_type.clone(),
                        uri: uri.clone(),
                    }))
                }
                _ => None,
            })
            .collect::<Result<Vec<_>, _>>()?;

        let rendition_reports = tags
            .iter()
            .filter_map(|t| match t {
                Tag::ExtXRenditionReport(attributes) => {
                    let Some(uri) = attributes.get("URI") else {
                        return Some(Err("no URI attribute found"));
                    };
                    let Some(last_msn) = attributes.get("LAST-MSN") else {
                        return Some(Err("no LAST-MSN attribute found"));
                    };
                    let Ok(last_msn) = last_msn.parse() else {
                        return Some(Err("LAST-MSN attribute is not a number"));
                    };
                    let Some(last_part) = attributes.get("LAST-PART") else {
                        return Some(Err("no LAST-PART attribute found"));
                    };
                    let Ok(last_part) = last_part.parse() else {
                        return Some(Err("LAST-PART attribute is not a number"));
                    };

                    Some(Ok(RenditionReport {
                        uri: uri.clone(),
                        last_msn,
                        last_part,
                    }))
                }
                _ => None,
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            version,
            target_duration,
            media_sequence,
            discontinuity_sequence,
            end_list,
            server_control,
            segments,
            preload_hint,
            part_target_duration,
            rendition_reports,
        })
    }
}
