#[derive(Debug, sqlx::Type, Default, Clone, Copy, PartialEq)]
#[sqlx(type_name = "playback_session_device")]
pub enum PlaybackSessionDevice {
	#[sqlx(rename = "UNKNOWN")]
	#[default]
	Unknown,
}

impl From<PlaybackSessionDevice> for pb::scuffle::video::v1::types::playback_session::Device {
	fn from(value: PlaybackSessionDevice) -> Self {
		match value {
			PlaybackSessionDevice::Unknown => Self::UnknownDevice,
		}
	}
}
