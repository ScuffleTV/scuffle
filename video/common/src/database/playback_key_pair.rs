use std::collections::HashMap;

use super::Ulid;

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct PlaybackKeyPair {
    pub id: Ulid,
    pub organization_id: Ulid,
    pub public_key: Vec<u8>,
    pub fingerprint: String,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub tags: sqlx::types::Json<HashMap<String, String>>,
}

impl PlaybackKeyPair {
    pub fn into_proto(self) -> pb::scuffle::video::v1::types::PlaybackKeyPair {
        pb::scuffle::video::v1::types::PlaybackKeyPair {
            id: Some(self.id.0.into()),
            fingerprint: self.fingerprint,
            created_at: self.id.0.timestamp_ms() as i64,
            updated_at: self.updated_at.timestamp_millis(),
            tags: Some(self.tags.0.into()),
        }
    }
}
