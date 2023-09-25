use crate::tests::global::mock_global_state;

use crate::database::{session, user};
use serial_test::serial;

#[serial]
#[tokio::test]
async fn test_serial_user_by_username_loader() {
    let (global, _) = mock_global_state(Default::default()).await;

    sqlx::query!("DELETE FROM users")
        .execute(global.db.as_ref())
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
        .fetch_one(global.db.as_ref())
        .await
        .unwrap();

    let session = sqlx::query_as!(
        session::Model,
        "INSERT INTO sessions(user_id, expires_at) VALUES ($1, $2) RETURNING *",
        user.id,
        chrono::Utc::now() + chrono::Duration::seconds(30)
    )
    .fetch_one(global.db.as_ref())
    .await
    .unwrap();

    let loaded = global
        .session_by_id_loader
        .load_one(session.id)
        .await
        .unwrap();

    assert!(loaded.is_some());

    let loaded = loaded.unwrap();
    assert_eq!(loaded.id, session.id);
    assert_eq!(loaded.user_id, session.user_id);
    assert_eq!(loaded.expires_at, session.expires_at);
    assert_eq!(loaded.created_at, session.created_at);
    assert_eq!(loaded.invalidated_at, session.invalidated_at);
    assert_eq!(loaded.last_used_at, session.last_used_at);
}
