use common::database::Ulid;
use pb::scuffle::video::v1::types::{playback_session, playback_session_target, PlaybackSessionTarget};

use super::playback_session_browser::PlaybackSessionBrowser;
use super::playback_session_device::PlaybackSessionDevice;
use super::playback_session_platform::PlaybackSessionPlatform;
use super::DatabaseTable;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PlaybackSession {
	pub id: Ulid,
	pub organization_id: Ulid,
	pub room_id: Option<Ulid>,
	pub recording_id: Option<Ulid>,
	pub user_id: Option<String>,
	pub playback_key_pair_id: Option<Ulid>,
	pub issued_at: Option<chrono::DateTime<chrono::Utc>>,
	pub expires_at: chrono::DateTime<chrono::Utc>,
	pub ip_address: String,
	pub user_agent: Option<String>,
	pub referer: Option<String>,
	pub origin: Option<String>,
	pub device: PlaybackSessionDevice,
	pub platform: PlaybackSessionPlatform,
	pub browser: PlaybackSessionBrowser,
	pub player_version: Option<String>,
}

impl DatabaseTable for PlaybackSession {
	const FRIENDLY_NAME: &'static str = "playback session";
	const NAME: &'static str = "playback_sessions";
}

impl PlaybackSession {
	pub fn into_proto(self) -> pb::scuffle::video::v1::types::PlaybackSession {
		pb::scuffle::video::v1::types::PlaybackSession {
			id: Some(self.id.0.into()),
			target: Some(PlaybackSessionTarget {
				target: Some(match self.room_id {
					Some(room_id) => playback_session_target::Target::RoomId(room_id.0.into()),
					None => playback_session_target::Target::RecordingId(self.recording_id.unwrap().0.into()),
				}),
			}),
			user_id: self.user_id,
			playback_key_pair_id: self.playback_key_pair_id.map(|id| id.0.into()),
			issued_at: self.issued_at.map(|dt| dt.timestamp_millis()),
			created_at: self.id.0.timestamp_ms() as i64,
			last_active_at: (self.expires_at - chrono::Duration::minutes(10)).timestamp_millis(),
			ip_address: self.ip_address,
			user_agent: self.user_agent,
			referer: self.referer,
			origin: self.origin,
			device: playback_session::Device::from(self.device).into(),
			platform: playback_session::Platform::from(self.platform).into(),
			browser: playback_session::Browser::from(self.browser).into(),
			player_version: self.player_version,
		}
	}
}
