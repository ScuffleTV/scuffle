use pb::scuffle::video::v1::types::playback_session;
use ulid::Ulid;
use uuid::Uuid;

use super::{
    playback_session_browser::PlaybackSessionBrowser,
    playback_session_device::PlaybackSessionDevice,
    playback_session_platform::PlaybackSessionPlatform,
};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PlaybackSession {
    pub id: Uuid,
    pub room_id: Option<Uuid>,
    pub recording_id: Option<Uuid>,
    pub organization_id: Uuid,
    pub user_id: Option<String>,
    pub playback_key_pair_id: Option<Uuid>,
    pub issued_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_active_at: chrono::DateTime<chrono::Utc>,
    pub ip_address: String,
    pub user_agent: Option<String>,
    pub referer: Option<String>,
    pub origin: Option<String>,
    pub device: PlaybackSessionDevice,
    pub platform: PlaybackSessionPlatform,
    pub browser: PlaybackSessionBrowser,
    pub player_version: Option<String>,
}

impl PlaybackSession {
    pub fn into_proto(self) -> pb::scuffle::video::v1::types::PlaybackSession {
        pb::scuffle::video::v1::types::PlaybackSession {
            id: Some(self.id.into()),
            target: if let Some(room_id) = self.room_id {
                Some(playback_session::Target::RoomId(room_id.into()))
            } else {
                self.recording_id
                    .map(|recording_id| playback_session::Target::RecordingId(recording_id.into()))
            },
            user_id: self.user_id,
            playback_key_pair_id: self.playback_key_pair_id.map(|id| id.into()),
            issued_at: self.issued_at.map(|dt| dt.timestamp_millis()),
            created_at: Ulid::from(self.id).timestamp_ms() as i64,
            last_active_at: self.last_active_at.timestamp_millis(),
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
