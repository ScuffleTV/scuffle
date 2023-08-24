use std::collections::HashMap;

use pb::scuffle::video::v1::types::Rendition as PbRendition;

use ulid::Ulid;
use uuid::Uuid;

use super::rendition::Rendition;

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct TranscodingConfig {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub renditions: Vec<Rendition>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub tags: Vec<String>,
}

impl TranscodingConfig {
    pub fn into_proto(self) -> pb::scuffle::video::v1::types::TranscodingConfig {
        pb::scuffle::video::v1::types::TranscodingConfig {
            id: Some(self.id.into()),
            renditions: self
                .renditions
                .into_iter()
                .map(|r| PbRendition::from(r).into())
                .collect(),
            created_at: Ulid::from(self.id).timestamp_ms() as i64,
            updated_at: self.updated_at.timestamp_micros(),
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
