use std::collections::HashMap;

use ulid::Ulid;
use uuid::Uuid;

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct PlaybackKeyPair {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub public_key: Vec<u8>,
    pub fingerprint: String,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub tags: Vec<String>,
}

impl PlaybackKeyPair {
    pub fn into_proto(self) -> pb::scuffle::video::v1::types::PlaybackKeyPair {
        pb::scuffle::video::v1::types::PlaybackKeyPair {
            id: Some(self.id.into()),
            fingerprint: self.fingerprint,
            created_at: Ulid::from(self.id).timestamp_ms() as i64,
            updated_at: self.updated_at.timestamp_millis(),
            tags: self
                .tags
                .iter()
                .map(|s| {
                    let splits = s.splitn(2, ':').collect::<Vec<_>>();

                    if splits.len() == 2 {
                        (splits[0].to_string(), splits[1].to_string())
                    } else {
                        (splits[0].to_string(), "".to_string())
                    }
                })
                .collect::<HashMap<_, _>>(),
        }
    }
}
