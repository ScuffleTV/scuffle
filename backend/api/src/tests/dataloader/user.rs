use crate::tests::global::mock_global_state;

use common::types::user;

#[tokio::test]
async fn test_user_by_username_loader() {
    let (global, _) = mock_global_state(Default::default()).await;

    sqlx::query!("DELETE FROM users")
        .execute(&*global.db)
        .await
        .unwrap();
    let user =
        sqlx::query_as!(user::Model,
        "INSERT INTO users(id, username, email, password_hash) VALUES ($1, $2, $3, $4) RETURNING *",
        1,
        "admin",
        "admin@admin.com",
        user::hash_password("admin")
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

#[tokio::test]
async fn test_user_by_id_loader() {
    let (global, _) = mock_global_state(Default::default()).await;

    sqlx::query!("DELETE FROM users")
        .execute(&*global.db)
        .await
        .unwrap();
    let user =
        sqlx::query_as!(user::Model,
        "INSERT INTO users(id, username, email, password_hash) VALUES ($1, $2, $3, $4) RETURNING *",
        1,
        "admin",
        "admin@admin.com",
        user::hash_password("admin")
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
