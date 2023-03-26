pub mod master;
pub mod media;

mod utils;

use std::str::FromStr;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub enum Playlist {
    Master(master::MasterPlaylist),
    Media(media::MediaPlaylist),
}

impl FromStr for Playlist {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let tags = utils::parse_tags(s)?;

        if tags.is_empty() {
            return Err("no tags found".to_string());
        }

        if tags.first().unwrap() != &utils::Tag::ExtM3u {
            return Err("first tag is not #EXTM3U".to_string());
        }

        if tags.iter().all(|t| t.is_master_tag()) {
            Ok(Playlist::Master(master::MasterPlaylist::from_tags(tags)?))
        } else {
            Ok(Playlist::Media(media::MediaPlaylist::from_tags(tags)?))
        }
    }
}

impl TryFrom<&[u8]> for Playlist {
    type Error = String;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let s = std::str::from_utf8(value).map_err(|_| "invalid bytes found in stream")?;
        s.parse()
    }
}
