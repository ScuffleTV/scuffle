#[derive(Debug, sqlx::Type, Default, Clone, Copy, PartialEq)]
#[sqlx(type_name = "playback_session_browser")]
pub enum PlaybackSessionBrowser {
    #[sqlx(rename = "UNKNOWN")]
    #[default]
    Unknown,
}

impl From<PlaybackSessionBrowser> for pb::scuffle::video::v1::types::playback_session::Browser {
    fn from(browser: PlaybackSessionBrowser) -> Self {
        match browser {
            PlaybackSessionBrowser::Unknown => Self::UnknownBrowser,
        }
    }
}
