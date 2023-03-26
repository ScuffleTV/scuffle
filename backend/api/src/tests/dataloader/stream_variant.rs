use crate::tests::global::mock_global_state;

use crate::database::{stream, stream_variant, user};
use chrono::Utc;
use serde_json::json;
use serial_test::serial;
use uuid::Uuid;

#[serial]
#[tokio::test]
async fn test_serial_stream_varariants_by_stream_id_loader() {
    let (global, _) = mock_global_state(Default::default()).await;

    sqlx::query!("DELETE FROM users")
        .execute(&*global.db)
        .await
        .unwrap();
    sqlx::query!("DELETE FROM streams")
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
    ).fetch_one(&*global.db).await.unwrap();

    let variants = vec![
        stream_variant::Model {
            id: Uuid::new_v4(),
            name: "video-audio".to_string(),
            stream_id: s.id,
            audio_bitrate: Some(128),
            audio_channels: Some(2),
            audio_sample_rate: Some(44100),
            video_bitrate: Some(12800),
            video_framerate: Some(30),
            video_height: Some(720),
            video_width: Some(1280),
            audio_codec: Some("aac".to_string()),
            video_codec: Some("h264".to_string()),
            created_at: Utc::now(),
            metadata: json!({}),
        },
        stream_variant::Model {
            id: Uuid::new_v4(),
            name: "video-only".to_string(),
            stream_id: s.id,
            audio_bitrate: None,
            audio_channels: None,
            audio_sample_rate: None,
            video_bitrate: Some(12800),
            video_framerate: Some(30),
            video_height: Some(720),
            video_width: Some(1280),
            audio_codec: None,
            video_codec: Some("h264".to_string()),
            created_at: Utc::now(),
            metadata: json!({}),
        },
        stream_variant::Model {
            id: Uuid::new_v4(),
            name: "audio-only".to_string(),
            stream_id: s.id,
            audio_bitrate: Some(128),
            audio_channels: Some(2),
            audio_sample_rate: Some(44100),
            video_bitrate: None,
            video_framerate: None,
            video_height: None,
            video_width: None,
            audio_codec: Some("aac".to_string()),
            video_codec: None,
            created_at: Utc::now(),
            metadata: json!({}),
        },
    ];

    for v in &variants {
        sqlx::query!("INSERT INTO stream_variants (id, name, stream_id, audio_bitrate, audio_channels, audio_sample_rate, video_bitrate, video_framerate, video_height, video_width, audio_codec, video_codec, created_at, metadata) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11,$12, $13, $14)",
            v.id,
            v.name,
            v.stream_id,
            v.audio_bitrate,
            v.audio_channels,
            v.audio_sample_rate,
            v.video_bitrate,
            v.video_framerate,
            v.video_height,
            v.video_width,
            v.audio_codec,
            v.video_codec,
            v.created_at,
            v.metadata,
        ).execute(&*global.db).await.unwrap();
    }

    let loaded = global
        .stream_variants_by_stream_id_loader
        .load_one(s.id)
        .await
        .unwrap();

    assert!(loaded.is_some());

    let loaded = loaded.unwrap();

    let audio_video = loaded.iter().find(|v| v.name == "video-audio").unwrap();

    assert_eq!(audio_video.audio_bitrate, Some(128));
    assert_eq!(audio_video.audio_channels, Some(2));
    assert_eq!(audio_video.audio_sample_rate, Some(44100));
    assert_eq!(audio_video.video_bitrate, Some(12800));
    assert_eq!(audio_video.video_framerate, Some(30));
    assert_eq!(audio_video.video_height, Some(720));
    assert_eq!(audio_video.video_width, Some(1280));
    assert_eq!(audio_video.audio_codec, Some("aac".to_string()));
    assert_eq!(audio_video.video_codec, Some("h264".to_string()));
    assert_eq!(audio_video.stream_id, s.id);
    assert_eq!(
        audio_video.created_at.timestamp(),
        variants[0].created_at.timestamp()
    );
    assert_eq!(audio_video.metadata, variants[0].metadata);

    let video_only = loaded.iter().find(|v| v.name == "video-only").unwrap();

    assert_eq!(video_only.audio_bitrate, None);
    assert_eq!(video_only.audio_channels, None);
    assert_eq!(video_only.audio_sample_rate, None);
    assert_eq!(video_only.video_bitrate, Some(12800));
    assert_eq!(video_only.video_framerate, Some(30));
    assert_eq!(video_only.video_height, Some(720));
    assert_eq!(video_only.video_width, Some(1280));
    assert_eq!(video_only.audio_codec, None);
    assert_eq!(video_only.video_codec, Some("h264".to_string()));
    assert_eq!(video_only.stream_id, s.id);
    assert_eq!(
        video_only.created_at.timestamp(),
        variants[1].created_at.timestamp()
    );
    assert_eq!(video_only.metadata, variants[1].metadata);

    let audio_only = loaded.iter().find(|v| v.name == "audio-only").unwrap();

    assert_eq!(audio_only.audio_bitrate, Some(128));
    assert_eq!(audio_only.audio_channels, Some(2));
    assert_eq!(audio_only.audio_sample_rate, Some(44100));
    assert_eq!(audio_only.video_bitrate, None);
    assert_eq!(audio_only.video_framerate, None);
    assert_eq!(audio_only.video_height, None);
    assert_eq!(audio_only.video_width, None);
    assert_eq!(audio_only.audio_codec, Some("aac".to_string()));
    assert_eq!(audio_only.video_codec, None);
    assert_eq!(audio_only.stream_id, s.id);
    assert_eq!(
        audio_only.created_at.timestamp(),
        variants[2].created_at.timestamp()
    );
    assert_eq!(audio_only.metadata, variants[2].metadata);
}
