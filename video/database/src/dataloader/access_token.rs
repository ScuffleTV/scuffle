use std::{collections::HashMap, sync::Arc};

use async_graphql::dataloader::Loader;
use async_trait::async_trait;
use ulid::Ulid;
use uuid::Uuid;

use crate::access_token::AccessToken;

pub struct AccessTokenByNameLoader {
    db: Arc<sqlx::PgPool>,
}

impl AccessTokenByNameLoader {
    pub fn new(db: Arc<sqlx::PgPool>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl Loader<Ulid> for AccessTokenByNameLoader {
    type Value = AccessToken;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, keys: &[Ulid]) -> Result<HashMap<Ulid, Self::Value>, Self::Error> {
        let query: Vec<Self::Value> = sqlx::query_as(
            r#"
            SELECT * FROM access_tokens WHERE id = ANY($1::uuid[])
        "#,
        )
        .bind(keys.iter().map(|id| Uuid::from(*id)).collect::<Vec<_>>())
        .fetch_all(self.db.as_ref())
        .await
        .map_err(Arc::new)?;

        let mut map = HashMap::new();
        for access_token in query {
            map.insert(access_token.id.into(), access_token);
        }

        Ok(map)
    }
}

pub struct AccessTokenUsedByNameUpdater {
    db: Arc<sqlx::PgPool>,
}

impl AccessTokenUsedByNameUpdater {
    pub fn new(db: Arc<sqlx::PgPool>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl Loader<Ulid> for AccessTokenUsedByNameUpdater {
    type Value = ();
    type Error = Arc<sqlx::Error>;

    async fn load(&self, keys: &[Ulid]) -> Result<HashMap<Ulid, Self::Value>, Self::Error> {
        sqlx::query(
            r#"
            UPDATE access_token SET last_active_at = NOW() WHERE id = ANY($1::uuid[])
        "#,
        )
        .bind(keys.iter().map(|id| Uuid::from(*id)).collect::<Vec<_>>())
        .execute(self.db.as_ref())
        .await
        .map_err(Arc::new)?;

        Ok(HashMap::new())
    }
}
