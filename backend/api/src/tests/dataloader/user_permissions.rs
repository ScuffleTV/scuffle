use crate::tests::global::mock_global_state;

use crate::database::{global_role::Permission, user};
use serial_test::serial;

#[serial]
#[tokio::test]
async fn test_serial_permissions_loader() {
    let (global, _) = mock_global_state(Default::default()).await;

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

    let no_go_live_role_id = sqlx::query!(
        "INSERT INTO global_roles(name, description, rank, allowed_permissions, denied_permissions, created_at) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
        "no_go_live",
        "no_go_live",
        3,
        0,
        Permission::GoLive.bits(),
        chrono::Utc::now()
    )
        .map(|row| row.id)
        .fetch_one(&*global.db)
        .await
        .unwrap();

    let no_admin_role_id = sqlx::query!(
        "INSERT INTO global_roles(name, description, rank, allowed_permissions, denied_permissions, created_at) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
        "no_admin",
        "no_admin",
        0,
        0,
        Permission::Admin.bits(),
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

    sqlx::query!(
        "INSERT INTO global_role_grants(user_id, global_role_id, created_at) VALUES ($1, $2, $3)",
        user_id,
        no_go_live_role_id,
        chrono::Utc::now()
    )
    .execute(&*global.db)
    .await
    .unwrap();

    sqlx::query!(
        "INSERT INTO global_role_grants(user_id, global_role_id, created_at) VALUES ($1, $2, $3)",
        user_id,
        no_admin_role_id,
        chrono::Utc::now()
    )
    .execute(&*global.db)
    .await
    .unwrap();

    let loaded = global
        .user_permisions_by_id_loader
        .load_one(user_id)
        .await
        .unwrap();
    assert!(loaded.is_some());

    let loaded = loaded.unwrap();
    assert_eq!(loaded.permissions, Permission::Admin);
    assert_eq!(loaded.user_id, user_id);

    assert_eq!(loaded.roles[0].id, no_admin_role_id);
    assert_eq!(loaded.roles[0].name, "no_admin");
    assert_eq!(loaded.roles[0].description, "no_admin");
    assert_eq!(loaded.roles[0].rank, 0);
    assert_eq!(loaded.roles[0].allowed_permissions, 0);
    assert_eq!(loaded.roles[0].denied_permissions, Permission::Admin);

    assert_eq!(loaded.roles[1].id, admin_role_id);
    assert_eq!(loaded.roles[1].name, "admin");
    assert_eq!(loaded.roles[1].description, "admin");
    assert_eq!(loaded.roles[1].rank, 1);
    assert_eq!(loaded.roles[1].allowed_permissions, Permission::Admin);
    assert_eq!(loaded.roles[1].denied_permissions, 0);

    assert_eq!(loaded.roles[2].id, go_live_role_id);
    assert_eq!(loaded.roles[2].name, "go_live");
    assert_eq!(loaded.roles[2].description, "go_live");
    assert_eq!(loaded.roles[2].rank, 2);
    assert_eq!(loaded.roles[2].allowed_permissions, Permission::GoLive);
    assert_eq!(loaded.roles[2].denied_permissions, 0);

    assert_eq!(loaded.roles[3].id, no_go_live_role_id);
    assert_eq!(loaded.roles[3].name, "no_go_live");
    assert_eq!(loaded.roles[3].description, "no_go_live");
    assert_eq!(loaded.roles[3].rank, 3);
    assert_eq!(loaded.roles[3].allowed_permissions, 0);
    assert_eq!(loaded.roles[3].denied_permissions, Permission::GoLive);
}

#[serial]
#[tokio::test]
async fn test_serial_permissions_loader_default_role() {
    let (global, _) = mock_global_state(Default::default()).await;

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

    let default_role_id = sqlx::query!(
        "INSERT INTO global_roles(name, description, rank, allowed_permissions, denied_permissions, created_at) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
        "default",
        "default",
        -1,
        Permission::GoLive.bits(),
        0,
        chrono::Utc::now()
    )
        .map(|row| row.id)
        .fetch_one(&*global.db)
        .await
        .unwrap();

    let loaded = global
        .user_permisions_by_id_loader
        .load_one(user_id)
        .await
        .unwrap();
    assert!(loaded.is_some());

    let loaded = loaded.unwrap();
    assert_eq!(loaded.permissions, Permission::GoLive);
    assert_eq!(loaded.user_id, user_id);

    assert_eq!(loaded.roles[0].id, default_role_id);
    assert_eq!(loaded.roles[0].name, "default");
    assert_eq!(loaded.roles[0].description, "default");
    assert_eq!(loaded.roles[0].rank, -1);
    assert_eq!(loaded.roles[0].allowed_permissions, Permission::GoLive);
    assert_eq!(loaded.roles[0].denied_permissions, 0);
}
