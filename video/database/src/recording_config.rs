use std::collections::HashMap;

use pb::scuffle::video::v1::types::{RecordingLifecyclePolicy, Rendition as PbRendition};
use ulid::Ulid;
use uuid::Uuid;

use crate::rendition::Rendition;

use super::adapter::Adapter;

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct RecordingConfig {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub renditions: Vec<Rendition>,
    pub lifecycle_policies: Vec<Adapter<RecordingLifecyclePolicy>>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub tags: Vec<String>,
}

impl RecordingConfig {
    pub fn into_proto(self) -> pb::scuffle::video::v1::types::RecordingConfig {
        pb::scuffle::video::v1::types::RecordingConfig {
            id: Some(self.id.into()),
            renditions: self
                .renditions
                .into_iter()
                .map(|r| PbRendition::from(r).into())
                .collect(),
            lifecycle_policies: self.lifecycle_policies.into_iter().map(|p| p.0).collect(),
            created_at: Ulid::from(self.id).timestamp_ms() as i64,
            updated_at: self.updated_at.timestamp_millis(),
            tags: self.tags.iter().map(|s| {
                let splits = s.splitn(2, ':').collect::<Vec<_>>();

                if splits.len() == 2 {
                    (splits[0].to_string(), splits[1].to_string())
                } else {
                    (splits[0].to_string(), "".to_string())
                }
            }).collect::<HashMap<_, _>>(),
        }
    }
}
