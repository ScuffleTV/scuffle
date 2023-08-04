use std::{collections::HashMap, sync::Arc};

use async_graphql::dataloader::Loader;
use async_trait::async_trait;
use uuid::Uuid;

use crate::organization::Organization;

pub struct OrganizationByIdLoader {
    db: Arc<sqlx::PgPool>,
}

#[async_trait]
impl Loader<Uuid> for OrganizationByIdLoader {
    type Value = Organization;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        let query: Vec<Self::Value> = sqlx::query_as(
            r#"
            SELECT * FROM organizations WHERE id = ANY($1)
        "#,
        )
        .bind(keys)
        .fetch_all(self.db.as_ref())
        .await
        .map_err(Arc::new)?;

        let mut map = HashMap::new();
        for organization in query {
            map.insert(organization.id, organization);
        }

        Ok(map)
    }
}
