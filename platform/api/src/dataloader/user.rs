use crate::database::user;
use async_graphql::{
    async_trait::async_trait,
    dataloader::{DataLoader, Loader},
};
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;

pub struct UserByUsernameLoader {
    db: Arc<sqlx::PgPool>,
}

impl UserByUsernameLoader {
    pub fn new(db: Arc<sqlx::PgPool>) -> DataLoader<Self> {
        DataLoader::new(Self { db }, tokio::spawn)
    }
}

#[async_trait]
impl Loader<String> for UserByUsernameLoader {
    type Value = user::Model;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, keys: &[String]) -> Result<HashMap<String, Self::Value>, Self::Error> {
        let results: Vec<user::Model> =
            sqlx::query_as("SELECT * FROM users WHERE username = ANY($1)")
                .bind(keys)
                .fetch_all(&*self.db)
                .await?;

        let mut map = HashMap::new();

        for result in results {
            map.insert(result.username.clone(), result);
        }

        Ok(map)
    }
}

pub struct UserByIdLoader {
    db: Arc<sqlx::PgPool>,
}

impl UserByIdLoader {
    pub fn new(db: Arc<sqlx::PgPool>) -> DataLoader<Self> {
        DataLoader::new(Self { db }, tokio::spawn)
    }
}

#[async_trait]
impl Loader<Uuid> for UserByIdLoader {
    type Value = user::Model;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        let results: Vec<user::Model> = sqlx::query_as("SELECT * FROM users WHERE id = ANY($1)")
            .bind(keys)
            .fetch_all(&*self.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch users: {}", e);
                Arc::new(e)
            })?;

        let mut map = HashMap::new();

        for result in results {
            map.insert(result.id, result);
        }

        Ok(map)
    }
}

pub struct UserSearchLoader {
    db: Arc<sqlx::PgPool>,
}

impl UserSearchLoader {
    pub fn new(db: Arc<sqlx::PgPool>) -> DataLoader<Self> {
        DataLoader::new(Self { db }, tokio::spawn)
    }
}

#[async_trait]
impl Loader<String> for UserSearchLoader {
    type Value = Vec<user::SearchResult>;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, keys: &[String]) -> Result<HashMap<String, Self::Value>, Self::Error> {
        let mut map = HashMap::new();

        for key in keys {
            let results: Vec<user::SearchResult> = sqlx::query_as(
                "SELECT users.*, similarity(username, $1) FROM users WHERE username % $1 ORDER BY similarity DESC LIMIT 5"
            )
            .bind(key)
            .fetch_all(&*self.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch users: {}", e);
                Arc::new(e)
            })?;

            map.insert(key.clone(), results);
        }

        Ok(map)
    }
}
