use crate::tests::global::mock_global_state;

use crate::database::user;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn test_serial_user_by_username_loader() {
    let (global, _) = mock_global_state(Default::default()).await;

    sqlx::query!("DELETE FROM users")
        .execute(&*global.db)
        .await
        .unwrap();
    let user =
        sqlx::query_as!(user::Model,
        "INSERT INTO users(username, display_name, email, password_hash, stream_key) VALUES ($1, $1, $2, $3, $4) RETURNING *",
        "admin",
        "admin@admin.com",
        user::hash_password("admin"),
        user::generate_stream_key(),
    )
        .fetch_one(&*global.db)
        .await
        .unwrap();

    let loaded = global
        .user_by_username_loader
        .load_one(user.username.clone())
        .await
        .unwrap();

    assert!(loaded.is_some());

    let loaded = loaded.unwrap();
    assert_eq!(loaded.id, user.id);
    assert_eq!(loaded.username, user.username);
    assert_eq!(loaded.email, user.email);
    assert_eq!(loaded.password_hash, user.password_hash);
    assert_eq!(loaded.created_at, user.created_at);
}

#[serial]
#[tokio::test]
async fn test_serial_user_by_id_loader() {
    let (global, _) = mock_global_state(Default::default()).await;

    sqlx::query!("DELETE FROM users")
        .execute(&*global.db)
        .await
        .unwrap();
    let user =
        sqlx::query_as!(user::Model,
        "INSERT INTO users(username, display_name, email, password_hash, stream_key) VALUES ($1, $1, $2, $3, $4) RETURNING *",
        "admin",
        "admin@admin.com",
        user::hash_password("admin"),
        user::generate_stream_key(),
    )
        .fetch_one(&*global.db)
        .await
        .unwrap();

    let loaded = global.user_by_id_loader.load_one(user.id).await.unwrap();

    assert!(loaded.is_some());

    let loaded = loaded.unwrap();
    assert_eq!(loaded.id, user.id);
    assert_eq!(loaded.username, user.username);
    assert_eq!(loaded.email, user.email);
    assert_eq!(loaded.password_hash, user.password_hash);
    assert_eq!(loaded.created_at, user.created_at);
}
