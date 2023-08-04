use pb::scuffle::video::v1::{
    types::ModifyMode, RoomGetRequest, RoomModifyRequest, RoomResetKeyRequest,
};
use sqlx::Execute;
use uuid::Uuid;
use video_database::access_token::AccessToken;

use crate::grpc::room::utils::room_reset_key_query;

use super::utils::{room_get_query, room_modify_query};

#[test]
fn test_room_modify_query_no_fields() {
    let mut request = RoomModifyRequest {
        name: "test".to_string(),
        mode: ModifyMode::Update.into(),
        ..Default::default()
    };

    let access_token = AccessToken {
        organization_id: Uuid::new_v4(),
        ..Default::default()
    };

    let error = match room_modify_query(&request, &access_token) {
        Err(error) => error,
        Ok(_) => panic!("expected error"),
    };

    assert_eq!(error.code(), tonic::Code::InvalidArgument);
    assert_eq!(
        error.message(),
        "no fields to update, please specify at least one field to update"
    );

    request.mode = ModifyMode::Create.into();

    let mut qb = room_modify_query(&request, &access_token).unwrap();
    let query = qb.build();

    assert_eq!(
        query.sql(),
        "INSERT INTO room (organization_id, name) VALUES ($1, $2) RETURNING *"
    );

    request.mode = ModifyMode::Upsert.into();

    let mut qb = room_modify_query(&request, &access_token).unwrap();
    let query = qb.build();

    assert_eq!(
        query.sql(),
        "INSERT INTO room (organization_id, name) VALUES ($1, $2) ON CONFLICT (organization_id, name) DO UPDATE SET updated_at = NOW() RETURNING *"
    );
}

#[test]
fn test_room_modify_query_1_field() {
    let cases = vec![
        (
            ModifyMode::Update,
            "UPDATE room SET private = $1, updated_at = NOW() WHERE organization_id = $2 AND name = $3 RETURNING *"
        ),
        (
            ModifyMode::Create,
            "INSERT INTO room (organization_id, name, private) VALUES ($1, $2, $3) RETURNING *"
        ),
        (
            ModifyMode::Upsert,
            "INSERT INTO room (organization_id, name, private) VALUES ($1, $2, $3) ON CONFLICT (organization_id, name) DO UPDATE SET private = EXCLUDED.private, updated_at = NOW() RETURNING *"
        ),
    ];

    for (mode, expected) in cases {
        let request = RoomModifyRequest {
            mode: mode as i32,
            name: "test".to_string(),
            private: Some(true),
            recording_config_name: None,
            transcoding_config_name: None,
        };

        let access_token = AccessToken {
            organization_id: Uuid::new_v4(),
            ..Default::default()
        };

        let mut qb = room_modify_query(&request, &access_token).unwrap();
        let query = qb.build();

        assert_eq!(query.sql(), expected);
    }
}

#[test]
fn test_room_modify_query_2_fields() {
    let cases = vec![
        (
            ModifyMode::Update,
            "UPDATE room SET recording_config_name = $1, private = $2, updated_at = NOW() WHERE organization_id = $3 AND name = $4 RETURNING *"
        ),
        (
            ModifyMode::Create,
            "INSERT INTO room (organization_id, name, recording_config_name, private) VALUES ($1, $2, $3, $4) RETURNING *"
        ),
        (
            ModifyMode::Upsert,
            "INSERT INTO room (organization_id, name, recording_config_name, private) VALUES ($1, $2, $3, $4) ON CONFLICT (organization_id, name) DO UPDATE SET recording_config_name = EXCLUDED.recording_config_name, private = EXCLUDED.private, updated_at = NOW() RETURNING *"
        ),
    ];

    for (mode, expected) in cases {
        let request = RoomModifyRequest {
            mode: mode as i32,
            name: "test".to_string(),
            private: Some(true),
            recording_config_name: Some("test".to_string()),
            transcoding_config_name: None,
        };

        let access_token = AccessToken {
            organization_id: Uuid::new_v4(),
            ..Default::default()
        };

        let mut qb = room_modify_query(&request, &access_token).unwrap();
        let query = qb.build();

        assert_eq!(query.sql(), expected);
    }
}

#[test]
fn test_room_modify_query_all_fields() {
    let cases = vec![
        (
            ModifyMode::Update,
            "UPDATE room SET transcoding_config_name = $1, recording_config_name = $2, private = $3, updated_at = NOW() WHERE organization_id = $4 AND name = $5 RETURNING *"
        ),
        (
            ModifyMode::Create,
            "INSERT INTO room (organization_id, name, transcoding_config_name, recording_config_name, private) VALUES ($1, $2, $3, $4, $5) RETURNING *"
        ),
        (
            ModifyMode::Upsert,
            "INSERT INTO room (organization_id, name, transcoding_config_name, recording_config_name, private) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (organization_id, name) DO UPDATE SET transcoding_config_name = EXCLUDED.transcoding_config_name, recording_config_name = EXCLUDED.recording_config_name, private = EXCLUDED.private, updated_at = NOW() RETURNING *"
        ),
    ];

    for (mode, expected) in cases {
        let request = RoomModifyRequest {
            mode: mode as i32,
            name: "test".to_string(),
            private: Some(true),
            recording_config_name: Some("test".to_string()),
            transcoding_config_name: Some("test".to_string()),
        };

        let access_token = AccessToken {
            organization_id: Uuid::new_v4(),
            ..Default::default()
        };

        let mut qb = room_modify_query(&request, &access_token).unwrap();
        let query = qb.build();

        assert_eq!(query.sql(), expected);
    }
}

#[test]
fn test_room_modify_query_invalid_name() {
    let request = RoomModifyRequest {
        mode: ModifyMode::Create.into(),
        name: "..".to_string(),
        private: None,
        recording_config_name: None,
        transcoding_config_name: None,
    };

    let access_token = AccessToken {
        organization_id: Uuid::new_v4(),
        ..Default::default()
    };

    let error = match room_modify_query(&request, &access_token) {
        Err(e) => e,
        Ok(_) => panic!("expected error"),
    };

    assert_eq!(error.code(), tonic::Code::InvalidArgument);
    assert_eq!(
        error.message(),
        "invalid room name, names must match ^[a-zA-Z0-9_-]{1,32}$"
    );
}

#[test]
fn test_room_modify_query_invalid_transcoding_name() {
    let request = RoomModifyRequest {
        mode: ModifyMode::Create.into(),
        name: "test".to_string(),
        private: None,
        recording_config_name: None,
        transcoding_config_name: Some("..".to_string()),
    };

    let access_token = AccessToken {
        organization_id: Uuid::new_v4(),
        ..Default::default()
    };

    let error = match room_modify_query(&request, &access_token) {
        Err(e) => e,
        Ok(_) => panic!("expected error"),
    };

    assert_eq!(error.code(), tonic::Code::InvalidArgument);
    assert_eq!(
        error.message(),
        "invalid transcoding_config_name, names must match ^[a-zA-Z0-9_-]{1,32}$"
    );
}

#[test]
fn test_room_modify_query_invalid_recording_name() {
    let request = RoomModifyRequest {
        mode: ModifyMode::Create.into(),
        name: "test".to_string(),
        private: None,
        transcoding_config_name: None,
        recording_config_name: Some("..".to_string()),
    };

    let access_token = AccessToken {
        organization_id: Uuid::new_v4(),
        ..Default::default()
    };

    let error = match room_modify_query(&request, &access_token) {
        Err(e) => e,
        Ok(_) => panic!("expected error"),
    };

    assert_eq!(error.code(), tonic::Code::InvalidArgument);
    assert_eq!(
        error.message(),
        "invalid recording_config_name, names must match ^[a-zA-Z0-9_-]{1,32}$"
    );
}

#[test]
fn test_room_modify_null_fields() {
    let cases = vec![
        (
            ModifyMode::Update,
            "UPDATE room SET transcoding_config_name = NULL, recording_config_name = NULL, updated_at = NOW() WHERE organization_id = $1 AND name = $2 RETURNING *"
        ),
        (
            ModifyMode::Create,
            "INSERT INTO room (organization_id, name, transcoding_config_name, recording_config_name) VALUES ($1, $2, NULL, NULL) RETURNING *"
        ),
        (
            ModifyMode::Upsert,
            "INSERT INTO room (organization_id, name, transcoding_config_name, recording_config_name) VALUES ($1, $2, NULL, NULL) ON CONFLICT (organization_id, name) DO UPDATE SET transcoding_config_name = EXCLUDED.transcoding_config_name, recording_config_name = EXCLUDED.recording_config_name, updated_at = NOW() RETURNING *"
        ),
    ];

    for (mode, expected) in cases {
        let request = RoomModifyRequest {
            mode: mode as i32,
            name: "test".to_string(),
            private: None,
            recording_config_name: Some("".into()),
            transcoding_config_name: Some("".into()),
        };

        let access_token = AccessToken {
            organization_id: Uuid::new_v4(),
            ..Default::default()
        };

        let mut qb = room_modify_query(&request, &access_token).unwrap();
        let query = qb.build();

        assert_eq!(query.sql(), expected);
    }
}

#[test]
fn test_room_get_query_live() {
    let access_token = AccessToken {
        organization_id: Uuid::new_v4(),
        ..Default::default()
    };

    let request = RoomGetRequest {
        limit: 10,
        live: Some(false),
        name: vec![],
        private: None,
        created_at: None,
        recording_config_name: None,
        transcoding_config_name: None,
    };

    let mut qb = room_get_query(&request, &access_token).unwrap();

    let query = qb.build();

    assert_eq!(
        query.sql(),
        "SELECT * FROM room WHERE organization_id = $1 AND live = $2 ORDER BY created_at LIMIT $3"
    );
}

#[test]
fn test_room_get_query_created_at() {
    let access_token = AccessToken {
        organization_id: Uuid::new_v4(),
        ..Default::default()
    };

    let request = RoomGetRequest {
        limit: 10,
        live: None,
        name: vec![],
        private: None,
        created_at: Some(chrono::Utc::now().timestamp_micros()),
        recording_config_name: None,
        transcoding_config_name: None,
    };

    let mut qb = room_get_query(&request, &access_token).unwrap();

    let query = qb.build();

    assert_eq!(
        query.sql(),
        "SELECT * FROM room WHERE organization_id = $1 AND created_at > $2 ORDER BY created_at LIMIT $3"
    );
}

#[test]
fn test_room_get_query_recording_name() {
    let access_token = AccessToken {
        organization_id: Uuid::new_v4(),
        ..Default::default()
    };

    let request = RoomGetRequest {
        limit: 1000,
        live: None,
        name: vec![],
        private: None,
        created_at: None,
        recording_config_name: Some("test".to_string()),
        transcoding_config_name: None,
    };

    let mut qb = room_get_query(&request, &access_token).unwrap();

    let query = qb.build();

    assert_eq!(
        query.sql(),
        "SELECT * FROM room WHERE organization_id = $1 AND recording_config_name = $2 ORDER BY created_at LIMIT $3"
    );
}

#[test]
fn test_room_get_query_transcoding_name() {
    let access_token = AccessToken {
        organization_id: Uuid::new_v4(),
        ..Default::default()
    };

    let request = RoomGetRequest {
        limit: 1000,
        live: None,
        name: vec![],
        private: None,
        created_at: None,
        recording_config_name: None,
        transcoding_config_name: Some("test".to_string()),
    };

    let mut qb = room_get_query(&request, &access_token).unwrap();

    let query = qb.build();

    assert_eq!(
        query.sql(),
        "SELECT * FROM room WHERE organization_id = $1 AND transcoding_config_name = $2 ORDER BY created_at LIMIT $3"
    );
}

#[test]
fn test_room_get_query_name() {
    let access_token = AccessToken {
        organization_id: Uuid::new_v4(),
        ..Default::default()
    };

    let request = RoomGetRequest {
        limit: 1000,
        live: None,
        name: vec!["test".to_string()],
        private: None,
        created_at: None,
        recording_config_name: None,
        transcoding_config_name: None,
    };

    let mut qb = room_get_query(&request, &access_token).unwrap();

    let query = qb.build();

    assert_eq!(
        query.sql(),
        "SELECT * FROM room WHERE organization_id = $1 AND name = ANY($2::text[]) ORDER BY created_at LIMIT $3"
    );
}

#[test]
fn test_room_get_query_private() {
    let access_token = AccessToken {
        organization_id: Uuid::new_v4(),
        ..Default::default()
    };

    let request = RoomGetRequest {
        limit: 1000,
        live: None,
        name: vec![],
        private: Some(true),
        created_at: None,
        recording_config_name: None,
        transcoding_config_name: None,
    };

    let mut qb = room_get_query(&request, &access_token).unwrap();

    let query = qb.build();

    assert_eq!(
        query.sql(),
        "SELECT * FROM room WHERE organization_id = $1 AND private = $2 ORDER BY created_at LIMIT $3"
    );
}

#[test]
fn test_room_get_query_nothing_but_limit() {
    let access_token = AccessToken {
        organization_id: Uuid::new_v4(),
        ..Default::default()
    };

    let request = RoomGetRequest {
        limit: 1000,
        live: None,
        name: vec![],
        private: None,
        created_at: None,
        recording_config_name: None,
        transcoding_config_name: None,
    };

    let mut qb = room_get_query(&request, &access_token).unwrap();

    let query = qb.build();

    assert_eq!(
        query.sql(),
        "SELECT * FROM room WHERE organization_id = $1 ORDER BY created_at LIMIT $2"
    );
}

#[test]
fn test_room_get_query_everything() {
    let access_token = AccessToken {
        organization_id: Uuid::new_v4(),
        ..Default::default()
    };

    let request = RoomGetRequest {
        limit: 1000,
        live: Some(false),
        name: vec!["test".to_string()],
        private: Some(true),
        created_at: Some(chrono::Utc::now().timestamp_micros()),
        recording_config_name: Some("test".to_string()),
        transcoding_config_name: Some("test".to_string()),
    };

    let mut qb = room_get_query(&request, &access_token).unwrap();

    let query = qb.build();

    assert_eq!(
        query.sql(),
        "SELECT * FROM room WHERE organization_id = $1 AND name = ANY($2::text[]) AND transcoding_config_name = $3 AND recording_config_name = $4 AND private = $5 AND live = $6 AND created_at > $7 ORDER BY created_at LIMIT $8"
    );
}

#[test]
fn test_room_get_query_null_names() {
    let access_token = AccessToken {
        organization_id: Uuid::new_v4(),
        ..Default::default()
    };

    let request = RoomGetRequest {
        limit: 1000,
        live: None,
        name: vec![],
        private: None,
        created_at: None,
        recording_config_name: Some("".into()),
        transcoding_config_name: Some("".into()),
    };

    let mut qb = room_get_query(&request, &access_token).unwrap();

    let query = qb.build();

    assert_eq!(
        query.sql(),
        "SELECT * FROM room WHERE organization_id = $1 AND transcoding_config_name IS NULL AND recording_config_name IS NULL ORDER BY created_at LIMIT $2"
    );
}

#[test]
fn test_room_get_query_no_limit() {
    let access_token = AccessToken {
        organization_id: Uuid::new_v4(),
        ..Default::default()
    };

    let request = RoomGetRequest::default();

    let mut qb = room_get_query(&request, &access_token).unwrap();

    let query = qb.build();

    assert_eq!(
        query.sql(),
        "SELECT * FROM room WHERE organization_id = $1 ORDER BY created_at LIMIT $2"
    );
}

#[test]
fn test_room_get_query_invalid_limit() {
    let access_token = AccessToken {
        organization_id: Uuid::new_v4(),
        ..Default::default()
    };

    let mut request = RoomGetRequest {
        limit: 1001,
        live: Some(false),
        name: vec!["test".to_string()],
        private: Some(true),
        created_at: Some(chrono::Utc::now().timestamp_micros()),
        recording_config_name: Some("test".to_string()),
        transcoding_config_name: Some("test".to_string()),
    };

    let error = match room_get_query(&request, &access_token) {
        Err(e) => e,
        Ok(_) => panic!("expected error"),
    };

    assert_eq!(error.code(), tonic::Code::InvalidArgument);
    assert_eq!(
        error.message(),
        "limit too large, must be between 1 and 1000"
    );

    request.limit = -1;

    let error = match room_get_query(&request, &access_token) {
        Err(e) => e,
        Ok(_) => panic!("expected error"),
    };

    assert_eq!(error.code(), tonic::Code::InvalidArgument);
    assert_eq!(
        error.message(),
        "limit too small, must be between 1 and 1000"
    );
}

#[test]
fn test_room_get_query_invalid_name() {
    let access_token = AccessToken {
        organization_id: Uuid::new_v4(),
        ..Default::default()
    };

    let mut request = RoomGetRequest {
        name: vec!["".to_string()],
        ..Default::default()
    };

    let error = match room_get_query(&request, &access_token) {
        Err(e) => e,
        Ok(_) => panic!("expected error"),
    };

    assert_eq!(error.code(), tonic::Code::InvalidArgument);
    assert_eq!(
        error.message(),
        "invalid name provided at index 0, names must match ^[a-zA-Z0-9_-]{1,32}$"
    );

    request.name = vec!["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()];

    let error = match room_get_query(&request, &access_token) {
        Err(e) => e,
        Ok(_) => panic!("expected error"),
    };

    assert_eq!(error.code(), tonic::Code::InvalidArgument);
    assert_eq!(
        error.message(),
        "invalid name provided at index 0, names must match ^[a-zA-Z0-9_-]{1,32}$"
    );

    // Non-ascii characters are not allowed
    request.name = vec!["😀".to_string()];

    let error = match room_get_query(&request, &access_token) {
        Err(e) => e,
        Ok(_) => panic!("expected error"),
    };

    assert_eq!(error.code(), tonic::Code::InvalidArgument);
    assert_eq!(
        error.message(),
        "invalid name provided at index 0, names must match ^[a-zA-Z0-9_-]{1,32}$"
    );

    // too many names is not allowed (max 100)
    request.name = vec!["test".to_string(); 101];

    let error = match room_get_query(&request, &access_token) {
        Err(e) => e,
        Ok(_) => panic!("expected error"),
    };

    assert_eq!(error.code(), tonic::Code::InvalidArgument);
    assert_eq!(error.message(), "too many names provided, max 100");
}

#[test]
fn test_room_get_query_invalid_recording_transcoding_name() {
    let access_token = AccessToken {
        organization_id: Uuid::new_v4(),
        ..Default::default()
    };

    let mut request = RoomGetRequest {
        transcoding_config_name: Some("..".into()),
        ..Default::default()
    };

    let error = match room_get_query(&request, &access_token) {
        Err(e) => e,
        Ok(_) => panic!("expected error"),
    };

    assert_eq!(error.code(), tonic::Code::InvalidArgument);
    assert_eq!(
        error.message(),
        "invalid transcoding_config_name provided, names must match ^[a-zA-Z0-9_-]{1,32}$"
    );

    request.transcoding_config_name = None;
    request.recording_config_name = Some("..".into());

    let error = match room_get_query(&request, &access_token) {
        Err(e) => e,
        Ok(_) => panic!("expected error"),
    };

    assert_eq!(error.code(), tonic::Code::InvalidArgument);
    assert_eq!(
        error.message(),
        "invalid recording_config_name provided, names must match ^[a-zA-Z0-9_-]{1,32}$"
    );
}

#[test]
fn test_room_get_query_invalid_created_at() {
    let access_token = AccessToken {
        organization_id: Uuid::new_v4(),
        ..Default::default()
    };

    let mut request = RoomGetRequest {
        // The request expects timestamps in microseconds so this will be way into the future
        created_at: Some(chrono::Utc::now().timestamp_nanos()),
        ..Default::default()
    };

    let error = match room_get_query(&request, &access_token) {
        Err(e) => e,
        Ok(_) => panic!("expected error"),
    };

    assert_eq!(error.code(), tonic::Code::InvalidArgument);
    assert_eq!(error.message(), "invalid created_at must be in the past");

    request.created_at = Some(-1);

    let error = match room_get_query(&request, &access_token) {
        Err(e) => e,
        Ok(_) => panic!("expected error"),
    };

    assert_eq!(error.code(), tonic::Code::InvalidArgument);
    assert_eq!(error.message(), "invalid created_at must be positive");
}

#[test]
fn test_room_reset_key_query() {
    let access_token = AccessToken {
        organization_id: Uuid::new_v4(),
        ..Default::default()
    };

    let mut request = RoomResetKeyRequest {
        names: vec!["test".to_string()],
    };

    let mut qb = room_reset_key_query(&request, &access_token).unwrap();

    let query = qb.build();

    assert_eq!(
        query.sql(),
        "UPDATE room as r SET stream_key = v.stream_key, updated_at = NOW() FROM (VALUES ($1, $2)) AS v(name, stream_key) WHERE r.organization_id = $3 AND r.name = v.name RETURNING r.*"
    );

    request.names = vec!["test".to_string(), "test2".to_string()];

    let mut qb = room_reset_key_query(&request, &access_token).unwrap();

    let query = qb.build();

    assert_eq!(
        query.sql(),
        "UPDATE room as r SET stream_key = v.stream_key, updated_at = NOW() FROM (VALUES ($1, $2), ($3, $4)) AS v(name, stream_key) WHERE r.organization_id = $5 AND r.name = v.name RETURNING r.*"
    );
}

#[test]
fn test_room_reset_key_query_invalid_names() {
    let access_token = AccessToken {
        organization_id: Uuid::new_v4(),
        ..Default::default()
    };

    let mut request = RoomResetKeyRequest {
        names: vec!["".to_string()],
    };

    let error = match room_reset_key_query(&request, &access_token) {
        Err(e) => e,
        Ok(_) => panic!("expected error"),
    };

    assert_eq!(error.code(), tonic::Code::InvalidArgument);
    assert_eq!(
        error.message(),
        "invalid room name provided at index 0, names must match ^[a-zA-Z0-9_-]{1,32}$"
    );

    request.names = vec!["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()];

    let error = match room_reset_key_query(&request, &access_token) {
        Err(e) => e,
        Ok(_) => panic!("expected error"),
    };

    assert_eq!(error.code(), tonic::Code::InvalidArgument);
    assert_eq!(
        error.message(),
        "invalid room name provided at index 0, names must match ^[a-zA-Z0-9_-]{1,32}$"
    );

    // Too many names not allowed (max 100)

    request.names = vec!["test".to_string(); 101];

    let error = match room_reset_key_query(&request, &access_token) {
        Err(e) => e,
        Ok(_) => panic!("expected error"),
    };

    assert_eq!(error.code(), tonic::Code::InvalidArgument);
    assert_eq!(error.message(), "too many names provided, max 100");

    request.names = vec![];

    let error = match room_reset_key_query(&request, &access_token) {
        Err(e) => e,
        Ok(_) => panic!("expected error"),
    };

    assert_eq!(error.code(), tonic::Code::InvalidArgument);
    assert_eq!(error.message(), "no names provided");
}
