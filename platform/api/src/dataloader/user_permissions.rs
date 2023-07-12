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

    async fn load(&self, keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        let default_role = sqlx::query_as!(
            global_role::Model,
            "SELECT * FROM global_roles WHERE rank = -1",
        )
        .fetch_optional(&*self.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch default role: {}", e);
            Arc::new(e)
        })?;

        let results = sqlx::query!(
            "SELECT rg.user_id, r.* FROM global_role_grants rg JOIN global_roles r ON rg.global_role_id = r.id WHERE rg.user_id = ANY($1) ORDER BY rg.user_id, r.rank ASC",
            &keys
        )
        .fetch_all(&*self.db)
        .await.map_err(|e| {
            tracing::error!("Failed to fetch user permissions: {}", e);
            Arc::new(e)
        })?;

        let mut map = HashMap::new();

        // We only care about the allowed_permissions, because the denied permissions only work on previous roles.
        // Since this is the first role, there are no previous roles, so the denied permissions are irrelevant.
        if let Some(default_role) = default_role {
            for key in keys {
                map.insert(
                    *key,
                    UserPermission {
                        user_id: *key,
                        permissions: default_role.allowed_permissions,
                        roles: vec![default_role.clone()],
                    },
                );
            }
        } else {
            for key in keys {
                map.insert(
                    *key,
                    UserPermission {
                        user_id: *key,
                        permissions: global_role::Permission::default(),
                        roles: Vec::new(),
                    },
                );
            }
        }

        for result in results {
            let current_user = map.entry(result.user_id).or_insert_with(|| UserPermission {
                user_id: result.user_id,
                permissions: global_role::Permission::default(),
                roles: Vec::new(),
            });

            current_user.permissions |= global_role::Permission::from(result.allowed_permissions);
            current_user.permissions &= !global_role::Permission::from(result.denied_permissions);

            current_user.roles.push(global_role::Model {
                id: result.id,
                name: result.name,
                description: result.description,
                allowed_permissions: result.allowed_permissions.into(),
                denied_permissions: result.denied_permissions.into(),
                created_at: result.created_at,
                rank: result.rank,
            });
        }

        Ok(map)
    }
}
