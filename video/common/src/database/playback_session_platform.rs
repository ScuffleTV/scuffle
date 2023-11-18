#[derive(Debug, sqlx::Type, Default, Clone, Copy, PartialEq)]
#[sqlx(type_name = "playback_session_platform")]
pub enum PlaybackSessionPlatform {
	#[sqlx(rename = "UNKNOWN")]
	#[default]
	Unknown,
}

impl From<PlaybackSessionPlatform> for pb::scuffle::video::v1::types::playback_session::Platform {
	fn from(value: PlaybackSessionPlatform) -> Self {
		match value {
			PlaybackSessionPlatform::Unknown => Self::UnknownPlatform,
		}
	}
}
