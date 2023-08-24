use std::collections::HashMap;

use chrono::Utc;
use ulid::Ulid;
use uuid::Uuid;

use pb::scuffle::video::v1::types::AccessTokenScope;

use super::adapter::Adapter;

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct AccessToken {
    pub organization_id: Uuid,
    pub id: Uuid,
    pub version: i32,
    pub updated_at: chrono::DateTime<Utc>,
    pub expires_at: Option<chrono::DateTime<Utc>>,
    pub last_active_at: Option<chrono::DateTime<Utc>>,
    pub scopes: Vec<Adapter<AccessTokenScope>>,
    pub tags: Vec<String>,
}

impl AccessToken {
    pub fn to_proto(self) -> pb::scuffle::video::v1::types::AccessToken {
        pb::scuffle::video::v1::types::AccessToken {
            id: Some(self.id.into()),
            created_at: Ulid::from(self.id).timestamp_ms() as i64,
            updated_at: self.updated_at.timestamp_millis(),
            expires_at: self.expires_at.map(|t| t.timestamp_millis()),
            last_used_at: self.last_active_at.map(|t| t.timestamp_millis()),
            scopes: self.scopes.into_iter().map(|s| s.0).collect(),
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
