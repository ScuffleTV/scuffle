use crate::api::v1::gql::{ext::RequestExt, request_context::RequestContext, schema};
use crate::database::{global_role::Permission, user};
use crate::tests::global::mock_global_state;
use async_graphql::{Name, Request, Value, Variables};
use common::prelude::FutureTimeout;
use serial_test::serial;
use std::sync::Arc;
use std::time::Duration;

#[serial]
#[tokio::test]
async fn test_serial_user_by_name() {
    let (global, handler) = mock_global_state(Default::default()).await;

    sqlx::query!("DELETE FROM users")
        .execute(&*global.db)
        .await
        .unwrap();

    sqlx::query!("DELETE FROM global_roles")
        .execute(&*global.db)
        .await
        .unwrap();

    sqlx::query!("DELETE FROM global_role_grants")
        .execute(&*global.db)
        .await
        .unwrap();

    let user_id = sqlx::query!(
        "INSERT INTO users(username, display_name, email, password_hash, stream_key) VALUES ($1, $1, $2, $3, $4) RETURNING id",
        "admin",
        "admin@admin.com",
        user::hash_password("admin"),
        user::generate_stream_key(),
    )
        .map(|row| row.id)
        .fetch_one(&*global.db)
        .await
        .unwrap();

    let admin_role_id = sqlx::query!(
        "INSERT INTO global_roles(name, description, rank, allowed_permissions, denied_permissions, created_at) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
        "admin",
        "admin",
        1,
        Permission::Admin.bits(),
        0,
        chrono::Utc::now()
    )
        .map(|row| row.id)
        .fetch_one(&*global.db)
        .await
        .unwrap();

    let go_live_role_id = sqlx::query!(
        "INSERT INTO global_roles(name, description, rank, allowed_permissions, denied_permissions, created_at) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
        "go_live",
        "go_live",
        2,
        Permission::GoLive.bits(),
        0,
        chrono::Utc::now()
    )
        .map(|row| row.id)
        .fetch_one(&*global.db)
        .await
        .unwrap();

    sqlx::query!(
        "INSERT INTO global_role_grants(user_id, global_role_id, created_at) VALUES ($1, $2, $3)",
        user_id,
        admin_role_id,
        chrono::Utc::now()
    )
    .execute(&*global.db)
    .await
    .unwrap();

    sqlx::query!(
        "INSERT INTO global_role_grants(user_id, global_role_id, created_at) VALUES ($1, $2, $3)",
        user_id,
        go_live_role_id,
        chrono::Utc::now()
    )
    .execute(&*global.db)
    .await
    .unwrap();

    let schema = schema();

    {
        let query = r#"
            query($id: UUID!) {
                userById(id: $id) {
                    id
                    permissions
                    globalRoles {
                        id
                        name
                        description
                        rank
                        allowedPermissions
                        deniedPermissions
                    }
                }
            }
        "#;

        let mut variables = Variables::default();
        variables.insert(Name::new("id"), Value::String(user_id.to_string()));

        let ctx = Arc::new(RequestContext::new(false));

        let res = tokio::time::timeout(
            Duration::from_secs(1),
            schema.execute(
                Request::from(query)
                    .variables(variables)
                    .provide_global(global.clone())
                    .provide_context(ctx),
            ),
        )
        .await
        .unwrap();

        assert!(res.is_ok());
        assert_eq!(res.errors.len(), 0);
        let json = res.data.into_json();
        assert!(json.is_ok());

        assert_eq!(
            json.unwrap(),
            serde_json::json!({
                "userById": {
                    "id": user_id,
                    "permissions": 3, // admin and go live permissions
                    "globalRoles": [
                        {
                            "id": admin_role_id,
                            "name": "admin",
                            "description": "admin",
                            "rank": 1,
                            "allowedPermissions": 1, // admin permission
                            "deniedPermissions": 0
                        },
                        {
                            "id": go_live_role_id,
                            "name": "go_live",
                            "description": "go_live",
                            "rank": 2,
                            "allowedPermissions": 2, // go live permission
                            "deniedPermissions": 0
                        },
                    ]
                }
            })
        );
    }

    drop(global);

    handler
        .cancel()
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel context");
}
