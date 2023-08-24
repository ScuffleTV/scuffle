use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Tag {
    ExtM3u,
    ExtXVersion(u8),
    ExtXIndependentSegments,
    ExtXStart(HashMap<String, String>),
    ExtXDefine(HashMap<String, String>),
    ExtXTargetDuration(u32),
    ExtXMediaSequence(u32),
    ExtXDiscontinuitySequence(u32),
    ExtXEndList,
    ExtXPlaylistType(PlaylistType),
    ExtXIFramesOnly,
    ExtXPartInf(HashMap<String, String>),
    ExtXServerControl(HashMap<String, String>),
    ExtInf(f64, String),
    ExtXByteRange(u32, Option<u32>),
    ExtXDiscontinuity,
    ExtXKey(HashMap<String, String>),
    ExtXMap(HashMap<String, String>),
    ExtXProgramDateTime(String),
    ExtXGap,
    ExtXBitrate(u32),
    ExtXPart(HashMap<String, String>),
    ExtXDateRange(HashMap<String, String>),
    ExtXSkip(HashMap<String, String>),
    ExtXPreloadHint(HashMap<String, String>),
    ExtXRenditionReport(HashMap<String, String>),
    ExtXMedia(HashMap<String, String>),
    ExtXStreamInf(HashMap<String, String>, String),
    ExtXIFrameStreamInf(HashMap<String, String>),
    ExtXSessionData(HashMap<String, String>),
    ExtXSessionKey(HashMap<String, String>),
    ExtXContentSteering(HashMap<String, String>),
    ExtXScufGroup(HashMap<String, String>),
    Unknown(String),
}

impl Tag {
    pub fn is_master_tag(&self) -> bool {
        matches!(
            self,
            Tag::ExtM3u
                | Tag::ExtXVersion(_)
                | Tag::ExtXIndependentSegments
                | Tag::ExtXStart(_)
                | Tag::ExtXDefine(_)
                | Tag::ExtXMedia(_)
                | Tag::ExtXStreamInf(_, _)
                | Tag::ExtXIFrameStreamInf(_)
                | Tag::ExtXSessionData(_)
                | Tag::ExtXSessionKey(_)
                | Tag::ExtXContentSteering(_)
                | Tag::ExtXScufGroup(_)
                | Tag::Unknown(_)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaylistType {
    Event,
    Vod,
}

pub fn parse_tags(input: &str) -> Result<Vec<Tag>, String> {
    // We first need to lazily parse the input into lines
    let mut lines = input.lines().filter(|l| !l.is_empty());

    let mut tags = Vec::new();
    while let Some(tag) = parse_tag(&mut lines)? {
        tags.push(tag);
    }

    Ok(tags)
}

fn parse_attributes(line: &str) -> Result<HashMap<String, String>, String> {
    let mut attributes = HashMap::new();

    let mut key = None;
    let mut value = String::new();

    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match c {
            '=' => {
                key = Some(value);
                value = String::new();
            }
            ',' => {
                let Some(key) = key.take() else { continue };

                attributes.insert(key, value);
                value = String::new();
            }
            '"' => {
                let mut value = String::new();

                while let Some(c) = chars.next() {
                    match c {
                        '"' => break,
                        '\\' => {
                            let c = chars.next().ok_or("invalid attribute2")?;

                            match c {
                                '"' => value.push('"'),
                                '\\' => value.push('\\'),
                                'n' => value.push('\n'),
                                'r' => value.push('\r'),
                                't' => value.push('\t'),
                                _ => return Err("invalid attribute3".into()),
                            }
                        }
                        _ => value.push(c),
                    }
                }

                let key = key.take().ok_or("invalid attribute4")?;
                attributes.insert(key, value);
            }
            c => {
                value.push(c);
            }
        }
    }

    if let Some(key) = key.take() {
        attributes.insert(key, value);
    }

    Ok(attributes)
}

fn parse_tag<'a>(lines: &mut impl Iterator<Item = &'a str>) -> Result<Option<Tag>, String> {
    let line = match lines.next() {
        Some(line) => line,
        None => return Ok(None),
    };

    match line {
        "#EXTM3U" => Ok(Some(Tag::ExtM3u)),
        line if line.starts_with("#EXT-X-VERSION:") => {
            let version = line
                .strip_prefix("#EXT-X-VERSION:")
                .ok_or("invalid version")?
                .parse()
                .map_err(|_| "invalid version")?;

            Ok(Some(Tag::ExtXVersion(version)))
        }
        _ if line.starts_with("#EXT-X-INDEPENDENT-SEGMENTS") => {
            Ok(Some(Tag::ExtXIndependentSegments))
        }
        line if line.starts_with("#EXT-X-START:") => {
            let attributes =
                parse_attributes(line.strip_prefix("#EXT-X-START:").ok_or("invalid start")?)?;

            Ok(Some(Tag::ExtXStart(attributes)))
        }
        line if line.starts_with("#EXT-X-DEFINE:") => {
            let attributes = parse_attributes(
                line.strip_prefix("#EXT-X-DEFINE:")
                    .ok_or("invalid define")?,
            )?;

            Ok(Some(Tag::ExtXDefine(attributes)))
        }
        line if line.starts_with("#EXT-X-TARGETDURATION:") => {
            let duration = line
                .strip_prefix("#EXT-X-TARGETDURATION:")
                .ok_or("invalid target duration")?
                .parse()
                .map_err(|_| "invalid target duration")?;

            Ok(Some(Tag::ExtXTargetDuration(duration)))
        }
        line if line.starts_with("#EXT-X-MEDIA-SEQUENCE:") => {
            let sequence = line
                .strip_prefix("#EXT-X-MEDIA-SEQUENCE:")
                .ok_or("invalid media sequence")?
                .parse()
                .map_err(|_| "invalid media sequence")?;

            Ok(Some(Tag::ExtXMediaSequence(sequence)))
        }
        line if line.starts_with("#EXT-X-DISCONTINUITY-SEQUENCE:") => {
            let sequence = line
                .strip_prefix("#EXT-X-DISCONTINUITY-SEQUENCE:")
                .ok_or("invalid discontinuity sequence")?
                .parse()
                .map_err(|_| "invalid discontinuity sequence")?;

            Ok(Some(Tag::ExtXDiscontinuitySequence(sequence)))
        }
        _ if line.starts_with("#EXT-X-ENDLIST") => Ok(Some(Tag::ExtXEndList)),
        line if line.starts_with("#EXT-X-PLAYLIST-TYPE:") => {
            let playlist_type = match line
                .strip_prefix("#EXT-X-PLAYLIST-TYPE:")
                .ok_or("invalid playlist type")?
                .to_uppercase()
                .as_str()
            {
                "EVENT" => PlaylistType::Event,
                "VOD" => PlaylistType::Vod,
                _ => return Err("invalid playlist type".to_string()),
            };

            Ok(Some(Tag::ExtXPlaylistType(playlist_type)))
        }
        _ if line.starts_with("#EXT-X-I-FRAMES-ONLY") => Ok(Some(Tag::ExtXIFramesOnly)),
        line if line.starts_with("#EXT-X-PART-INF:") => {
            let attributes = parse_attributes(
                line.strip_prefix("#EXT-X-PART-INF:")
                    .ok_or("invalid part inf")?,
            )?;

            Ok(Some(Tag::ExtXPartInf(attributes)))
        }
        line if line.starts_with("#EXT-X-SERVER-CONTROL:") => {
            let attributes = parse_attributes(
                line.strip_prefix("#EXT-X-SERVER-CONTROL:")
                    .ok_or("invalid server control")?,
            )?;

            Ok(Some(Tag::ExtXServerControl(attributes)))
        }
        line if line.starts_with("#EXT-X-SESSION-KEY:") => {
            let attributes = parse_attributes(
                line.strip_prefix("#EXT-X-SESSION-KEY:")
                    .ok_or("invalid session key")?,
            )?;

            Ok(Some(Tag::ExtXSessionKey(attributes)))
        }
        line if line.starts_with("#EXT-X-SESSION-DATA:") => {
            let attributes = parse_attributes(
                line.strip_prefix("#EXT-X-SESSION-DATA:")
                    .ok_or("invalid session data")?,
            )?;

            Ok(Some(Tag::ExtXSessionData(attributes)))
        }
        line if line.starts_with("#EXTINF:") => {
            let mut splits = line
                .strip_prefix("#EXTINF:")
                .ok_or("invalid duration")?
                .split(',');

            let duration = splits
                .next()
                .ok_or("invalid duration")?
                .parse()
                .map_err(|_| "invalid duration")?;

            Ok(Some(Tag::ExtInf(
                duration,
                lines.next().ok_or("invalid uri")?.into(),
            )))
        }
        line if line.starts_with("#EXT-X-BYTERANGE:") => {
            let mut splits = line
                .strip_prefix("#EXT-X-BYTERANGE:")
                .ok_or("invalid byterange")?
                .split('@');

            let length = splits
                .next()
                .ok_or("invalid byterange")?
                .parse()
                .map_err(|_| "invalid byterange")?;

            let offset = match splits
                .next()
                .map(|s| s.parse().map_err(|_| "invalid byterange"))
            {
                Some(Ok(offset)) => Some(offset),
                Some(Err(err)) => return Err(err.into()),
                None => None,
            };

            Ok(Some(Tag::ExtXByteRange(length, offset)))
        }
        _ if line.starts_with("#EXT-X-DISCONTINUITY") => Ok(Some(Tag::ExtXDiscontinuity)),
        line if line.starts_with("#EXT-X-KEY:") => {
            let attributes =
                parse_attributes(line.strip_prefix("#EXT-X-KEY:").ok_or("invalid key")?)?;

            Ok(Some(Tag::ExtXKey(attributes)))
        }
        line if line.starts_with("#EXT-X-MAP:") => {
            let attributes =
                parse_attributes(line.strip_prefix("#EXT-X-MAP:").ok_or("invalid map")?)?;

            Ok(Some(Tag::ExtXMap(attributes)))
        }
        line if line.starts_with("#EXT-X-PROGRAM-DATE-TIME:") => {
            let date_time = line
                .strip_prefix("#EXT-X-PROGRAM-DATE-TIME:")
                .ok_or("invalid program date time")?;

            Ok(Some(Tag::ExtXProgramDateTime(date_time.into())))
        }
        _ if line.starts_with("#EXT-X-GAP") => Ok(Some(Tag::ExtXGap)),
        line if line.starts_with("#EXT-X-BITRATE:") => {
            let bitrate = line
                .strip_prefix("#EXT-X-BITRATE:")
                .ok_or("invalid bitrate")?
                .parse()
                .map_err(|_| "invalid bitrate")?;

            Ok(Some(Tag::ExtXBitrate(bitrate)))
        }
        line if line.starts_with("#EXT-X-PART:") => {
            let attributes =
                parse_attributes(line.strip_prefix("#EXT-X-PART:").ok_or("invalid part")?)?;

            Ok(Some(Tag::ExtXPart(attributes)))
        }
        line if line.starts_with("#EXT-X-PRELOAD-HINT:") => {
            let attributes = parse_attributes(
                line.strip_prefix("#EXT-X-PRELOAD-HINT:")
                    .ok_or("invalid preload hint")?,
            )?;

            Ok(Some(Tag::ExtXPreloadHint(attributes)))
        }
        line if line.starts_with("#EXT-X-DATERANGE:") => {
            let attributes = parse_attributes(
                line.strip_prefix("#EXT-X-DATERANGE:")
                    .ok_or("invalid datarange")?,
            )?;

            Ok(Some(Tag::ExtXDateRange(attributes)))
        }
        line if line.starts_with("#EXT-X-SKIP:") => {
            let attributes =
                parse_attributes(line.strip_prefix("#EXT-X-SKIP:").ok_or("invalid skip")?)?;

            Ok(Some(Tag::ExtXSkip(attributes)))
        }
        line if line.starts_with("#EXT-X-RENDITION-REPORT:") => {
            let attributes = parse_attributes(
                line.strip_prefix("#EXT-X-RENDITION-REPORT:")
                    .ok_or("invalid rendition report")?,
            )?;

            Ok(Some(Tag::ExtXRenditionReport(attributes)))
        }
        line if line.starts_with("#EXT-X-MEDIA:") => {
            let attributes =
                parse_attributes(line.strip_prefix("#EXT-X-MEDIA:").ok_or("invalid media")?)?;

            Ok(Some(Tag::ExtXMedia(attributes)))
        }
        line if line.starts_with("#EXT-X-STREAM-INF:") => {
            let attributes = parse_attributes(
                line.strip_prefix("#EXT-X-STREAM-INF:")
                    .ok_or("invalid stream inf")?,
            )?;

            Ok(Some(Tag::ExtXStreamInf(
                attributes,
                lines.next().ok_or("invalid stream inf")?.into(),
            )))
        }
        line if line.starts_with("#EXT-X-I-FRAME-STREAM-INF:") => {
            let attributes = parse_attributes(
                line.strip_prefix("#EXT-X-I-FRAME-STREAM-INF:")
                    .ok_or("invalid iframe stream inf")?,
            )?;

            Ok(Some(Tag::ExtXIFrameStreamInf(attributes)))
        }
        line if line.starts_with("#EXT-X-SESSION-DATA:") => {
            let attributes = parse_attributes(
                line.strip_prefix("#EXT-X-SESSION-DATA:")
                    .ok_or("invalid session data")?,
            )?;

            Ok(Some(Tag::ExtXSessionData(attributes)))
        }
        line if line.starts_with("#EXT-X-SESSION-KEY:") => {
            let attributes = parse_attributes(
                line.strip_prefix("#EXT-X-SESSION-KEY:")
                    .ok_or("invalid session key")?,
            )?;

            Ok(Some(Tag::ExtXSessionKey(attributes)))
        }
        line if line.starts_with("#EXT-X-CONTENT-STEERING:") => {
            let attributes = parse_attributes(
                line.strip_prefix("#EXT-X-CONTENT-STEERING:")
                    .ok_or("invalid content steering")?,
            )?;

            Ok(Some(Tag::ExtXContentSteering(attributes)))
        }
        line if line.starts_with("#EXT-X-SCUF-GROUP:") => {
            let attributes = parse_attributes(
                line.strip_prefix("#EXT-X-SCUF-GROUP:")
                    .ok_or("invalid scuf group")?,
            )?;

            Ok(Some(Tag::ExtXScufGroup(attributes)))
        }
        line => Ok(Some(Tag::Unknown(line.into()))),
    }
}
