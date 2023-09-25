use crate::tests::global::mock_global_state;

use crate::database::{stream, user};
use serial_test::serial;
use uuid::Uuid;

#[serial]
#[tokio::test]
async fn test_serial_stream_by_id_loader() {
    let (global, _) = mock_global_state(Default::default()).await;

    sqlx::query!("DELETE FROM users")
        .execute(global.db.as_ref())
        .await
        .unwrap();
    sqlx::query!("DELETE FROM streams")
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

    let conn_id = Uuid::new_v4();
    let s = sqlx::query_as!(stream::Model,
        "INSERT INTO streams (channel_id, title, description, recorded, transcoded, ingest_address, connection_id) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *",
        user.id,
        "test",
        "test",
        false,
        false,
        "some address",
        conn_id,
    ).fetch_one(global.db.as_ref()).await.unwrap();

    let loaded = global.stream_by_id_loader.load_one(s.id).await.unwrap();

    assert!(loaded.is_some());

    let loaded = loaded.unwrap();
    assert_eq!(loaded.id, s.id);
    assert_eq!(loaded.channel_id, user.id);
    assert_eq!(loaded.title, "test");
    assert_eq!(loaded.description, "test");
    assert!(!loaded.recorded);
    assert!(!loaded.transcoded);
    assert_eq!(loaded.ingest_address, "some address");
    assert_eq!(loaded.connection_id, conn_id);
}
