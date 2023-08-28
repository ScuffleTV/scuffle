use crate::database::global_role;
use async_graphql::{
    async_trait::async_trait,
    dataloader::{DataLoader, Loader},
};
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;

pub struct UserPermissionsByIdLoader {
    db: Arc<sqlx::PgPool>,
}

impl UserPermissionsByIdLoader {
    pub fn new(db: Arc<sqlx::PgPool>) -> DataLoader<Self> {
        DataLoader::new(Self { db }, tokio::spawn)
    }
}

#[derive(Debug, Clone, Default)]
pub struct UserPermission {
    pub user_id: Uuid,
    pub permissions: global_role::Permission,
    pub roles: Vec<global_role::Model>,
}

#[async_trait]
impl Loader<Uuid> for UserPermissionsByIdLoader {
    type Value = UserPermission;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, _keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        let _default_role: Option<global_role::Model> =
            sqlx::query_as("SELECT * FROM global_roles WHERE rank = -1")
                .fetch_optional(&*self.db)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to fetch default role: {}", e);
                    Arc::new(e)
                })?;

        todo!("xd");

        Ok(HashMap::new())
    }
}
