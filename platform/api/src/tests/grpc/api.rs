use crate::config::{AppConfig, GrpcConfig};
use crate::database::{global_role::Permission, user};
use crate::database::{stream, stream_bitrate_update, stream_event};
use crate::grpc::run;
use crate::tests::global::mock_global_state;
use chrono::Utc;
use common::grpc::make_channel;
use common::prelude::FutureTimeout;
use pb::scuffle::internal::video::rpc::{
    update_live_stream_request, NewLiveStreamRequest, StreamReadyState,
};
use pb::scuffle::internal::video::types::{stream_state, StreamState};
use serial_test::serial;
use std::time::Duration;
use uuid::Uuid;

#[serial]
#[tokio::test]
async fn test_serial_grpc_authenticate_invalid_stream_key() {
    let port = portpicker::pick_unused_port().expect("failed to pick port");

    let (global, handler) = mock_global_state(AppConfig {
        grpc: GrpcConfig {
            bind_address: format!("0.0.0.0:{}", port).parse().unwrap(),
            ..Default::default()
        },
        ..Default::default()
    })
    .await;

    let handle = tokio::spawn(run(global));

    // We only want a single resolve attempt, so we set the timeout to 0
    let channel = make_channel(
        vec![format!("localhost:{}", port)],
        Duration::from_secs(0),
        None,
    )
    .unwrap();

    let mut client = pb::scuffle::internal::video::rpc::api_client::ApiClient::new(channel);
    let err = client
        .authenticate_live_stream(
            pb::scuffle::internal::video::rpc::AuthenticateLiveStreamRequest {
                app_name: "test".to_string(),
                stream_key: "test".to_string(),
                ip_address: "127.0.0.1".to_string(),
                ingest_address: "127.0.0.1:1234".to_string(),
                connection_id: Uuid::new_v4().to_string(),
            },
        )
        .await
        .unwrap_err();

    assert_eq!(err.code(), tonic::Code::InvalidArgument);
    assert_eq!(err.message(), "invalid stream key");

    handler
        .cancel()
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel context");

    handle
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel grpc")
        .expect("grpc failed")
        .expect("grpc failed");
}

#[serial]
#[tokio::test]
async fn test_serial_grpc_authenticate_valid_stream_key() {
    let port = portpicker::pick_unused_port().expect("failed to pick port");

    let (global, handler) = mock_global_state(AppConfig {
        grpc: GrpcConfig {
            bind_address: format!("0.0.0.0:{}", port).parse().unwrap(),
            ..Default::default()
        },
        ..Default::default()
    })
    .await;

    let db = global.db.clone();
    sqlx::query!("DELETE FROM users")
        .execute(&*db)
        .await
        .unwrap();
    sqlx::query!("DELETE FROM global_roles")
        .execute(&*db)
        .await
        .unwrap();
    sqlx::query!("DELETE FROM global_role_grants")
        .execute(&*db)
        .await
        .unwrap();

    let user = sqlx::query_as!(user::Model,
        "INSERT INTO users (username, display_name, email, password_hash, stream_key) VALUES ($1, $1, $2, $3, $4) RETURNING *",
        "test",
        "test@test.com",
        user::hash_password("test"),
        user::generate_stream_key(),
    ).fetch_one(&*db).await.unwrap();

    let go_live_role_id = sqlx::query!(
        "INSERT INTO global_roles(name, description, rank, allowed_permissions, denied_permissions, created_at) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
        "Go Live",
        "Allows a user to go live",
        0,
        Permission::GoLive.bits(),
        0,
        chrono::Utc::now(),
    ).map(|r| r.id).fetch_one(&*db).await.unwrap();

    let handle = tokio::spawn(run(global));

    let channel = make_channel(
        vec![format!("localhost:{}", port)],
        Duration::from_secs(0),
        None,
    )
    .unwrap();

    let mut client = pb::scuffle::internal::video::rpc::api_client::ApiClient::new(channel);
    let resp = client
        .authenticate_live_stream(
            pb::scuffle::internal::video::rpc::AuthenticateLiveStreamRequest {
                app_name: "test".to_string(),
                stream_key: user.get_stream_key(),
                ip_address: "127.0.0.1".to_string(),
                ingest_address: "127.0.0.1:1234".to_string(),
                connection_id: Uuid::new_v4().to_string(),
            },
        )
        .await
        .unwrap_err();

    assert_eq!(resp.code(), tonic::Code::PermissionDenied);
    assert_eq!(resp.message(), "user has no permission to go live");

    sqlx::query!(
        "INSERT INTO global_role_grants (user_id, global_role_id) VALUES ($1, $2)",
        user.id,
        go_live_role_id
    )
    .execute(&*db)
    .await
    .unwrap();

    let resp = client
        .authenticate_live_stream(
            pb::scuffle::internal::video::rpc::AuthenticateLiveStreamRequest {
                app_name: "test".to_string(),
                stream_key: user.get_stream_key(),
                ip_address: "127.0.0.1".to_string(),
                ingest_address: "127.0.0.1:1234".to_string(),
                connection_id: Uuid::new_v4().to_string(),
            },
        )
        .await
        .unwrap()
        .into_inner();

    assert!(!resp.record);
    assert!(!resp.transcode);
    assert!(!resp.stream_id.is_empty());

    handler
        .cancel()
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel context");

    handle
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel grpc")
        .expect("grpc failed")
        .expect("grpc failed");
}

#[serial]
#[tokio::test]
async fn test_serial_grpc_authenticate_valid_stream_key_ext() {
    let port = portpicker::pick_unused_port().expect("failed to pick port");

    let (global, handler) = mock_global_state(AppConfig {
        grpc: GrpcConfig {
            bind_address: format!("0.0.0.0:{}", port).parse().unwrap(),
            ..Default::default()
        },
        ..Default::default()
    })
    .await;

    let db = global.db.clone();
    sqlx::query!("DELETE FROM users")
        .execute(&*db)
        .await
        .unwrap();
    sqlx::query!("DELETE FROM global_roles")
        .execute(&*db)
        .await
        .unwrap();
    sqlx::query!("DELETE FROM global_role_grants")
        .execute(&*db)
        .await
        .unwrap();

    let user = sqlx::query_as!(user::Model,
        "INSERT INTO users (username, display_name, email, password_hash, stream_key, stream_recording_enabled, stream_transcoding_enabled) VALUES ($1, $1, $2, $3, $4, true, true) RETURNING *",
        "test",
        "test@test.com",
        user::hash_password("test"),
        user::generate_stream_key(),
    ).fetch_one(&*db).await.unwrap();

    let go_live_role_id = sqlx::query!(
        "INSERT INTO global_roles(name, description, rank, allowed_permissions, denied_permissions, created_at) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
        "Go Live",
        "Allows a user to go live",
        0,
        (Permission::GoLive | Permission::StreamRecording).bits(),
        0,
        chrono::Utc::now(),
    ).map(|r| r.id).fetch_one(&*db).await.unwrap();

    sqlx::query!(
        "INSERT INTO global_role_grants (user_id, global_role_id) VALUES ($1, $2)",
        user.id,
        go_live_role_id
    )
    .execute(&*db)
    .await
    .unwrap();

    let handle = tokio::spawn(run(global));
    let channel = make_channel(
        vec![format!("localhost:{}", port)],
        Duration::from_secs(0),
        None,
    )
    .unwrap();

    let mut client = pb::scuffle::internal::video::rpc::api_client::ApiClient::new(channel);

    let resp = client
        .authenticate_live_stream(
            pb::scuffle::internal::video::rpc::AuthenticateLiveStreamRequest {
                app_name: "test".to_string(),
                stream_key: user.get_stream_key(),
                ip_address: "127.0.0.1".to_string(),
                ingest_address: "127.0.0.1:1234".to_string(),
                connection_id: Uuid::new_v4().to_string(),
            },
        )
        .await
        .unwrap()
        .into_inner();

    assert!(resp.record);
    assert!(!resp.transcode);
    assert!(!resp.stream_id.is_empty());

    handler
        .cancel()
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel context");

    handle
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel grpc")
        .expect("grpc failed")
        .expect("grpc failed");
}

#[serial]
#[tokio::test]
async fn test_serial_grpc_authenticate_valid_stream_key_ext_2() {
    let port = portpicker::pick_unused_port().expect("failed to pick port");
    let (global, handler) = mock_global_state(AppConfig {
        grpc: GrpcConfig {
            bind_address: format!("0.0.0.0:{}", port).parse().unwrap(),
            ..Default::default()
        },
        ..Default::default()
    })
    .await;

    let db = global.db.clone();
    sqlx::query!("DELETE FROM users")
        .execute(&*db)
        .await
        .unwrap();
    sqlx::query!("DELETE FROM global_roles")
        .execute(&*db)
        .await
        .unwrap();
    sqlx::query!("DELETE FROM global_role_grants")
        .execute(&*db)
        .await
        .unwrap();

    let user = sqlx::query_as!(user::Model,
        "INSERT INTO users (username, display_name, email, password_hash, stream_key, stream_recording_enabled, stream_transcoding_enabled) VALUES ($1, $1, $2, $3, $4, true, true) RETURNING *",
        "test",
        "test@test.com",
        user::hash_password("test"),
        user::generate_stream_key(),
    ).fetch_one(&*db).await.unwrap();

    let go_live_role_id = sqlx::query!(
        "INSERT INTO global_roles(name, description, rank, allowed_permissions, denied_permissions, created_at) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
        "Go Live",
        "Allows a user to go live",
        0,
        (Permission::GoLive | Permission::StreamTranscoding).bits(),
        0,
        chrono::Utc::now(),
    ).map(|r| r.id).fetch_one(&*db).await.unwrap();

    sqlx::query!(
        "INSERT INTO global_role_grants (user_id, global_role_id) VALUES ($1, $2)",
        user.id,
        go_live_role_id
    )
    .execute(&*db)
    .await
    .unwrap();

    let handle = tokio::spawn(run(global));
    let channel = make_channel(
        vec![format!("localhost:{}", port)],
        Duration::from_secs(0),
        None,
    )
    .unwrap();

    let mut client = pb::scuffle::internal::video::rpc::api_client::ApiClient::new(channel);

    let resp = client
        .authenticate_live_stream(
            pb::scuffle::internal::video::rpc::AuthenticateLiveStreamRequest {
                app_name: "test".to_string(),
                stream_key: user.get_stream_key(),
                ip_address: "127.0.0.1".to_string(),
                ingest_address: "127.0.0.1:1234".to_string(),
                connection_id: Uuid::new_v4().to_string(),
            },
        )
        .await
        .unwrap()
        .into_inner();

    assert!(!resp.record);
    assert!(resp.transcode);
    assert!(!resp.stream_id.is_empty());

    handler
        .cancel()
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel context");

    handle
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel grpc")
        .expect("grpc failed")
        .expect("grpc failed");
}

#[serial]
#[tokio::test]
async fn test_serial_grpc_update_live_stream_state() {
    let port = portpicker::pick_unused_port().expect("failed to pick port");
    let (global, handler) = mock_global_state(AppConfig {
        grpc: GrpcConfig {
            bind_address: format!("0.0.0.0:{}", port).parse().unwrap(),
            ..Default::default()
        },
        ..Default::default()
    })
    .await;

    let db = global.db.clone();
    sqlx::query!("DELETE FROM users")
        .execute(&*db)
        .await
        .unwrap();
    sqlx::query!("DELETE FROM streams")
        .execute(&*db)
        .await
        .unwrap();

    let user = sqlx::query_as!(user::Model,
        "INSERT INTO users (username, display_name, email, password_hash, stream_key, stream_recording_enabled, stream_transcoding_enabled) VALUES ($1, $1, $2, $3, $4, true, true) RETURNING *",
        "test",
        "test@test.com",
        user::hash_password("test"),
        user::generate_stream_key(),
    ).fetch_one(&*db).await.unwrap();

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
    ).fetch_one(&*db).await.unwrap();

    let handle = tokio::spawn(run(global));
    let channel = make_channel(
        vec![format!("localhost:{}", port)],
        Duration::from_secs(0),
        None,
    )
    .unwrap();

    let mut client = pb::scuffle::internal::video::rpc::api_client::ApiClient::new(channel);

    {
        let timestamp = Utc::now().timestamp() as u64;

        assert!(client
            .update_live_stream(pb::scuffle::internal::video::rpc::UpdateLiveStreamRequest {
                connection_id: conn_id.to_string(),
                stream_id: s.id.to_string(),
                updates: vec![update_live_stream_request::Update {
                    timestamp,
                    update: Some(update_live_stream_request::update::Update::ReadyState(
                        StreamReadyState::Ready as i32
                    )),
                }]
            })
            .await
            .is_ok());

        let s = sqlx::query_as!(stream::Model, "SELECT * FROM streams WHERE id = $1", s.id,)
            .fetch_one(&*db)
            .await
            .unwrap();

        assert_eq!(s.ready_state, stream::ReadyState::Ready);
        assert_eq!(s.updated_at.unwrap().timestamp() as u64, timestamp);
    }

    {
        let timestamp = Utc::now().timestamp() as u64;

        assert!(client
            .update_live_stream(pb::scuffle::internal::video::rpc::UpdateLiveStreamRequest {
                connection_id: conn_id.to_string(),
                stream_id: s.id.to_string(),
                updates: vec![update_live_stream_request::Update {
                    timestamp,
                    update: Some(update_live_stream_request::update::Update::ReadyState(
                        StreamReadyState::NotReady as i32
                    )),
                }]
            })
            .await
            .is_ok());

        let s = sqlx::query_as!(stream::Model, "SELECT * FROM streams WHERE id = $1", s.id,)
            .fetch_one(&*db)
            .await
            .unwrap();

        assert_eq!(s.ready_state, stream::ReadyState::NotReady);
        assert_eq!(s.updated_at.unwrap().timestamp() as u64, timestamp);
    }

    {
        let timestamp = Utc::now().timestamp() as u64;

        assert!(client
            .update_live_stream(pb::scuffle::internal::video::rpc::UpdateLiveStreamRequest {
                connection_id: conn_id.to_string(),
                stream_id: s.id.to_string(),
                updates: vec![update_live_stream_request::Update {
                    timestamp,
                    update: Some(update_live_stream_request::update::Update::ReadyState(
                        StreamReadyState::Failed as i32
                    )),
                }]
            })
            .await
            .is_ok());

        let s = sqlx::query_as!(stream::Model, "SELECT * FROM streams WHERE id = $1", s.id,)
            .fetch_one(&*db)
            .await
            .unwrap();

        assert_eq!(s.ready_state, stream::ReadyState::Failed);
        assert_eq!(s.updated_at.unwrap().timestamp() as u64, timestamp);
        assert_eq!(s.ended_at.timestamp() as u64, timestamp);
    }

    for i in 0..2 {
        let timestamp = Utc::now().timestamp() as u64;

        let res = client
            .update_live_stream(pb::scuffle::internal::video::rpc::UpdateLiveStreamRequest {
                connection_id: conn_id.to_string(),
                stream_id: s.id.to_string(),
                updates: vec![update_live_stream_request::Update {
                    timestamp,
                    update: Some(update_live_stream_request::update::Update::ReadyState(
                        StreamReadyState::Stopped as i32,
                    )),
                }],
            })
            .await;

        if i == 0 {
            assert!(res.is_err());
            sqlx::query!(
                "UPDATE streams SET ready_state = 0, ended_at = $2 WHERE id = $1;",
                s.id,
                Utc::now() + chrono::Duration::seconds(300)
            )
            .execute(&*db)
            .await
            .unwrap();
        } else {
            assert!(res.is_ok());
            let s = sqlx::query_as!(stream::Model, "SELECT * FROM streams WHERE id = $1", s.id,)
                .fetch_one(&*db)
                .await
                .unwrap();

            assert_eq!(s.ready_state, stream::ReadyState::Stopped);
            assert_eq!(s.updated_at.unwrap().timestamp() as u64, timestamp);
            assert_eq!(s.ended_at.timestamp() as u64, timestamp);
        }
    }

    for i in 0..2 {
        let timestamp = Utc::now().timestamp() as u64;

        let res = client
            .update_live_stream(pb::scuffle::internal::video::rpc::UpdateLiveStreamRequest {
                connection_id: conn_id.to_string(),
                stream_id: s.id.to_string(),
                updates: vec![update_live_stream_request::Update {
                    timestamp,
                    update: Some(update_live_stream_request::update::Update::ReadyState(
                        StreamReadyState::StoppedResumable as i32,
                    )),
                }],
            })
            .await;

        if i == 0 {
            assert!(res.is_err());
            sqlx::query!(
                "UPDATE streams SET ready_state = 0, ended_at = $2 WHERE id = $1;",
                s.id,
                Utc::now() + chrono::Duration::seconds(300)
            )
            .execute(&*db)
            .await
            .unwrap();
        } else {
            assert!(res.is_ok());
            let s = sqlx::query_as!(stream::Model, "SELECT * FROM streams WHERE id = $1", s.id,)
                .fetch_one(&*db)
                .await
                .unwrap();

            assert_eq!(s.ready_state, stream::ReadyState::StoppedResumable);
            assert_eq!(s.updated_at.unwrap().timestamp() as u64, timestamp);
            assert_eq!(s.ended_at.timestamp() as u64, timestamp + 300);
        }
    }

    handler
        .cancel()
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel context");

    handle
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel grpc")
        .expect("grpc failed")
        .expect("grpc failed");
}

#[serial]
#[tokio::test]
async fn test_serial_grpc_update_live_stream_bitrate() {
    let port = portpicker::pick_unused_port().expect("failed to pick port");
    let (global, handler) = mock_global_state(AppConfig {
        grpc: GrpcConfig {
            bind_address: format!("0.0.0.0:{}", port).parse().unwrap(),
            ..Default::default()
        },
        ..Default::default()
    })
    .await;

    let db = global.db.clone();
    sqlx::query!("DELETE FROM users")
        .execute(&*db)
        .await
        .unwrap();
    sqlx::query!("DELETE FROM streams")
        .execute(&*db)
        .await
        .unwrap();

    let user = sqlx::query_as!(user::Model,
        "INSERT INTO users (username, display_name, email, password_hash, stream_key, stream_recording_enabled, stream_transcoding_enabled) VALUES ($1, $1, $2, $3, $4, true, true) RETURNING *",
        "test",
        "test@test.com",
        user::hash_password("test"),
        user::generate_stream_key(),
    ).fetch_one(&*db).await.unwrap();

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
    ).fetch_one(&*db).await.unwrap();

    let handle = tokio::spawn(run(global));
    let channel = make_channel(
        vec![format!("localhost:{}", port)],
        Duration::from_secs(0),
        None,
    )
    .unwrap();

    let mut client = pb::scuffle::internal::video::rpc::api_client::ApiClient::new(channel);

    {
        let timestamp = Utc::now().timestamp() as u64;

        assert!(client
            .update_live_stream(pb::scuffle::internal::video::rpc::UpdateLiveStreamRequest {
                connection_id: conn_id.to_string(),
                stream_id: s.id.to_string(),
                updates: vec![update_live_stream_request::Update {
                    timestamp,
                    update: Some(update_live_stream_request::update::Update::Bitrate(
                        update_live_stream_request::Bitrate {
                            video_bitrate: 1000,
                            audio_bitrate: 1000,
                            metadata_bitrate: 1000
                        }
                    )),
                }]
            })
            .await
            .is_ok());

        let s = sqlx::query_as!(
            stream_bitrate_update::Model,
            "SELECT * FROM stream_bitrate_updates WHERE stream_id = $1",
            s.id,
        )
        .fetch_one(&*db)
        .await
        .unwrap();

        assert_eq!(s.audio_bitrate, 1000);
        assert_eq!(s.video_bitrate, 1000);
        assert_eq!(s.metadata_bitrate, 1000);
        assert_eq!(s.created_at.timestamp() as u64, timestamp);
    }

    handler
        .cancel()
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel context");

    handle
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel grpc")
        .expect("grpc failed")
        .expect("grpc failed");
}

#[serial]
#[tokio::test]
async fn test_serial_grpc_update_live_stream_event() {
    let port = portpicker::pick_unused_port().expect("failed to pick port");
    let (global, handler) = mock_global_state(AppConfig {
        grpc: GrpcConfig {
            bind_address: format!("0.0.0.0:{}", port).parse().unwrap(),
            ..Default::default()
        },
        ..Default::default()
    })
    .await;

    let db = global.db.clone();
    sqlx::query!("DELETE FROM users")
        .execute(&*db)
        .await
        .unwrap();
    sqlx::query!("DELETE FROM streams")
        .execute(&*db)
        .await
        .unwrap();

    let user = sqlx::query_as!(user::Model,
        "INSERT INTO users (username, display_name, email, password_hash, stream_key, stream_recording_enabled, stream_transcoding_enabled) VALUES ($1, $1, $2, $3, $4, true, true) RETURNING *",
        "test",
        "test@test.com",
        user::hash_password("test"),
        user::generate_stream_key(),
    ).fetch_one(&*db).await.unwrap();

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
    ).fetch_one(&*db).await.unwrap();

    let handle = tokio::spawn(run(global));
    let channel = make_channel(
        vec![format!("localhost:{}", port)],
        Duration::from_secs(0),
        None,
    )
    .unwrap();

    let mut client = pb::scuffle::internal::video::rpc::api_client::ApiClient::new(channel);

    {
        let timestamp = Utc::now().timestamp() as u64;

        assert!(client
            .update_live_stream(pb::scuffle::internal::video::rpc::UpdateLiveStreamRequest {
                connection_id: conn_id.to_string(),
                stream_id: s.id.to_string(),
                updates: vec![update_live_stream_request::Update {
                    timestamp,
                    update: Some(update_live_stream_request::update::Update::Event(
                        update_live_stream_request::Event {
                            level: update_live_stream_request::event::Level::Info.into(),
                            message: "test - message".to_string(),
                            title: "test - title".to_string(),
                        }
                    )),
                }]
            })
            .await
            .is_ok());

        let s = sqlx::query_as!(
            stream_event::Model,
            "SELECT * FROM stream_events WHERE stream_id = $1",
            s.id,
        )
        .fetch_one(&*db)
        .await
        .unwrap();

        assert_eq!(s.level, stream_event::Level::Info);
        assert_eq!(s.message, "test - message");
        assert_eq!(s.title, "test - title");
        assert_eq!(s.created_at.timestamp() as u64, timestamp);
    }

    handler
        .cancel()
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel context");

    handle
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel grpc")
        .expect("grpc failed")
        .expect("grpc failed");
}

#[serial]
#[tokio::test]
async fn test_serial_grpc_update_live_stream_variants() {
    let port = portpicker::pick_unused_port().expect("failed to pick port");
    let (global, handler) = mock_global_state(AppConfig {
        grpc: GrpcConfig {
            bind_address: format!("0.0.0.0:{}", port).parse().unwrap(),
            ..Default::default()
        },
        ..Default::default()
    })
    .await;

    let db = global.db.clone();
    sqlx::query!("DELETE FROM users")
        .execute(&*db)
        .await
        .unwrap();
    sqlx::query!("DELETE FROM streams")
        .execute(&*db)
        .await
        .unwrap();

    let user = sqlx::query_as!(user::Model,
        "INSERT INTO users (username, display_name, email, password_hash, stream_key, stream_recording_enabled, stream_transcoding_enabled) VALUES ($1, $1, $2, $3, $4, true, true) RETURNING *",
        "test",
        "test@test.com",
        user::hash_password("test"),
        user::generate_stream_key(),
    ).fetch_one(&*db).await.unwrap();

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
    ).fetch_one(&*db).await.unwrap();

    let handle = tokio::spawn(run(global));
    let channel = make_channel(
        vec![format!("localhost:{}", port)],
        Duration::from_secs(0),
        None,
    )
    .unwrap();

    let mut client = pb::scuffle::internal::video::rpc::api_client::ApiClient::new(channel);

    {
        let timestamp = Utc::now().timestamp() as u64;

        let source_id = Uuid::new_v4().to_string();
        let audio_id = Uuid::new_v4().to_string();

        let stream_state = StreamState {
            variants: vec![
                stream_state::Variant {
                    name: "source".to_string(),
                    group: "aac".to_string(),
                    transcode_ids: vec![source_id.to_string(), audio_id.to_string()],
                },
                stream_state::Variant {
                    name: "audio-only".to_string(),
                    group: "aac".to_string(),
                    transcode_ids: vec![audio_id.to_string()],
                },
            ],
            transcodes: vec![
                stream_state::Transcode {
                    bitrate: 8000 * 1024,
                    codec: "avc1.640028".to_string(),
                    id: source_id.to_string(),
                    copy: true,
                    settings: Some(stream_state::transcode::Settings::Video(
                        stream_state::transcode::VideoSettings {
                            framerate: 60,
                            height: 1080,
                            width: 1920,
                        },
                    )),
                },
                stream_state::Transcode {
                    bitrate: 128 * 1024,
                    codec: "mp4a.40.2".to_string(),
                    id: audio_id.to_string(),
                    copy: false,
                    settings: Some(stream_state::transcode::Settings::Audio(
                        stream_state::transcode::AudioSettings {
                            channels: 2,
                            sample_rate: 48000,
                        },
                    )),
                },
            ],
            groups: vec![
                stream_state::Group {
                    name: "opus".to_string(),
                    priority: 1,
                },
                stream_state::Group {
                    name: "aac".to_string(),
                    priority: 2,
                },
            ],
        };

        assert!(client
            .update_live_stream(pb::scuffle::internal::video::rpc::UpdateLiveStreamRequest {
                connection_id: conn_id.to_string(),
                stream_id: s.id.to_string(),
                updates: vec![update_live_stream_request::Update {
                    timestamp,
                    update: Some(update_live_stream_request::update::Update::State(
                        stream_state.clone()
                    )),
                }]
            })
            .await
            .is_ok());

        let s = sqlx::query_as!(stream::Model, "SELECT * FROM streams WHERE id = $1", s.id,)
            .fetch_one(&*db)
            .await
            .unwrap();

        assert_eq!(s.state, Some(stream_state));
    }

    handler
        .cancel()
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel context");

    handle
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel grpc")
        .expect("grpc failed")
        .expect("grpc failed");
}

#[serial]
#[tokio::test]
async fn test_serial_grpc_new_live_stream() {
    let port = portpicker::pick_unused_port().expect("failed to pick port");
    let (global, handler) = mock_global_state(AppConfig {
        grpc: GrpcConfig {
            bind_address: format!("0.0.0.0:{}", port).parse().unwrap(),
            ..Default::default()
        },
        ..Default::default()
    })
    .await;

    let db = global.db.clone();
    sqlx::query!("DELETE FROM users")
        .execute(&*db)
        .await
        .unwrap();
    sqlx::query!("DELETE FROM streams")
        .execute(&*db)
        .await
        .unwrap();

    let user = sqlx::query_as!(user::Model,
        "INSERT INTO users (username, display_name, email, password_hash, stream_key, stream_recording_enabled, stream_transcoding_enabled) VALUES ($1, $1, $2, $3, $4, true, true) RETURNING *",
        "test",
        "test@test.com",
        user::hash_password("test"),
        user::generate_stream_key(),
    ).fetch_one(&*db).await.unwrap();

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
    ).fetch_one(&*db).await.unwrap();

    let handle = tokio::spawn(run(global));
    let channel = make_channel(
        vec![format!("localhost:{}", port)],
        Duration::from_secs(0),
        None,
    )
    .unwrap();

    let mut client = pb::scuffle::internal::video::rpc::api_client::ApiClient::new(channel);

    let source_id = Uuid::new_v4().to_string();
    let audio_id = Uuid::new_v4().to_string();

    let stream_state = StreamState {
        variants: vec![
            stream_state::Variant {
                name: "source".to_string(),
                group: "aac".to_string(),
                transcode_ids: vec![source_id.to_string(), audio_id.to_string()],
            },
            stream_state::Variant {
                name: "audio-only".to_string(),
                group: "aac".to_string(),
                transcode_ids: vec![audio_id.to_string()],
            },
        ],
        transcodes: vec![
            stream_state::Transcode {
                bitrate: 8000 * 1024,
                codec: "avc1.640028".to_string(),
                id: source_id.to_string(),
                copy: true,
                settings: Some(stream_state::transcode::Settings::Video(
                    stream_state::transcode::VideoSettings {
                        framerate: 60,
                        height: 1080,
                        width: 1920,
                    },
                )),
            },
            stream_state::Transcode {
                bitrate: 128 * 1024,
                codec: "mp4a.40.2".to_string(),
                id: audio_id.to_string(),
                copy: false,
                settings: Some(stream_state::transcode::Settings::Audio(
                    stream_state::transcode::AudioSettings {
                        channels: 2,
                        sample_rate: 48000,
                    },
                )),
            },
        ],
        groups: vec![stream_state::Group {
            name: "aac".to_string(),
            priority: 1,
        }],
    };

    let response = client
        .new_live_stream(NewLiveStreamRequest {
            old_stream_id: s.id.to_string(),
            state: Some(stream_state.clone()),
        })
        .await
        .unwrap()
        .into_inner();

    let s = sqlx::query_as!(stream::Model, "SELECT * FROM streams WHERE id = $1", s.id)
        .fetch_one(&*db)
        .await
        .unwrap();

    assert_eq!(s.channel_id, user.id);
    assert_eq!(s.title, "test");
    assert_eq!(s.description, "test");
    assert!(!s.recorded);
    assert!(!s.transcoded);
    assert_eq!(s.ingest_address, "some address");
    assert_eq!(s.connection_id, conn_id);
    assert_eq!(s.ready_state, stream::ReadyState::Stopped);

    let stream_id = Uuid::parse_str(&response.stream_id).unwrap();

    let s = sqlx::query_as!(
        stream::Model,
        "SELECT * FROM streams WHERE id = $1",
        stream_id,
    )
    .fetch_one(&*db)
    .await
    .unwrap();

    assert_eq!(s.channel_id, user.id);
    assert_eq!(s.title, "test");
    assert_eq!(s.description, "test");
    assert!(!s.recorded);
    assert!(!s.transcoded);
    assert_eq!(s.ingest_address, "some address");
    assert_eq!(s.connection_id, conn_id);
    assert_eq!(s.ready_state, stream::ReadyState::NotReady);
    assert_eq!(s.state, Some(stream_state));

    handler
        .cancel()
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel context");

    handle
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel grpc")
        .expect("grpc failed")
        .expect("grpc failed");
}
