use chrono::Utc;
use pb::scuffle::video::v1::{
    types::ModifyMode, RoomGetRequest, RoomModifyRequest, RoomResetKeyRequest,
};
use rand::Rng;
use sqlx::QueryBuilder;
use tonic::Status;
use video_database::access_token::AccessToken;

pub fn generate_stream_key() -> String {
    const VALID_CHARACTERS: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

    let mut rng = rand::thread_rng();

    let mut stream_key = String::with_capacity(32);

    for _ in 0..32 {
        let random = rng.gen_range(0..VALID_CHARACTERS.len()) as u8;

        stream_key.push(VALID_CHARACTERS.as_bytes()[random as usize] as char);
    }

    stream_key
}

pub fn room_modify_query<'a>(
    request: &'a RoomModifyRequest,
    access_token: &'a AccessToken,
) -> Result<QueryBuilder<'a, sqlx::Postgres>, Status> {
    let mut query_builder = QueryBuilder::<sqlx::Postgres>::default();

    if !video_database::name::validate(&request.name) {
        return Err(Status::invalid_argument(format!(
            "invalid room name, names must match {}",
            video_database::name::REGEX,
        )));
    }

    if let Some(recording_config_name) = &request.recording_config_name {
        if !recording_config_name.is_empty()
            && !video_database::name::validate(recording_config_name)
        {
            return Err(Status::invalid_argument(format!(
                "invalid recording_config_name, names must match {}",
                video_database::name::REGEX,
            )));
        }
    }

    if let Some(transcoding_config_name) = &request.transcoding_config_name {
        if !transcoding_config_name.is_empty()
            && !video_database::name::validate(transcoding_config_name)
        {
            return Err(Status::invalid_argument(format!(
                "invalid transcoding_config_name, names must match {}",
                video_database::name::REGEX,
            )));
        }
    }

    match request.mode() {
        ModifyMode::Update => {
            if request.private.is_none()
                && request.recording_config_name.is_none()
                && request.transcoding_config_name.is_none()
            {
                return Err(Status::invalid_argument(
                    "no fields to update, please specify at least one field to update",
                ));
            }

            query_builder.push("UPDATE room SET ");

            let mut separated = query_builder.separated(", ");

            if let Some(transcoding_config_name) = &request.transcoding_config_name {
                if transcoding_config_name.is_empty() {
                    separated.push("transcoding_config_name = NULL");
                } else {
                    separated.push("transcoding_config_name = ");
                    separated.push_bind_unseparated(transcoding_config_name);
                }
            }

            if let Some(recording_config_name) = &request.recording_config_name {
                if recording_config_name.is_empty() {
                    separated.push("recording_config_name = NULL");
                } else {
                    separated.push("recording_config_name = ");
                    separated.push_bind_unseparated(recording_config_name);
                }
            }

            if let Some(private) = request.private {
                separated.push("private = ");
                separated.push_bind_unseparated(private);
            }

            separated.push("updated_at = NOW()");

            query_builder.push(" WHERE organization_id = ");
            query_builder.push_bind(access_token.organization_id);
            query_builder.push(" AND name = ");
            query_builder.push_bind(&request.name);
        }
        ModifyMode::Create | ModifyMode::Upsert => {
            query_builder.push("INSERT INTO room (");

            let mut separated = query_builder.separated(", ");

            separated.push("organization_id");
            separated.push("name");

            if request.transcoding_config_name.is_some() {
                separated.push("transcoding_config_name");
            }

            if request.recording_config_name.is_some() {
                separated.push("recording_config_name");
            }

            if request.private.is_some() {
                separated.push("private");
            }

            separated.push_unseparated(") VALUES (");

            separated.push_bind_unseparated(access_token.organization_id);
            separated.push_bind(&request.name);

            if let Some(transcoding_config_name) = &request.transcoding_config_name {
                if transcoding_config_name.is_empty() {
                    separated.push("NULL");
                } else {
                    separated.push_bind(transcoding_config_name);
                }
            }

            if let Some(recording_config_name) = &request.recording_config_name {
                if recording_config_name.is_empty() {
                    separated.push("NULL");
                } else {
                    separated.push_bind(recording_config_name);
                }
            }

            if let Some(private) = request.private {
                separated.push_bind(private);
            }

            separated.push_unseparated(")");

            if request.mode() == ModifyMode::Upsert {
                query_builder.push(" ON CONFLICT (organization_id, name) DO UPDATE SET ");

                let mut separated = query_builder.separated(", ");

                if request.transcoding_config_name.is_some() {
                    separated.push("transcoding_config_name = EXCLUDED.transcoding_config_name");
                }

                if request.recording_config_name.is_some() {
                    separated.push("recording_config_name = EXCLUDED.recording_config_name");
                }

                if request.private.is_some() {
                    separated.push("private = EXCLUDED.private");
                }

                separated.push("updated_at = NOW()");
            }
        }
    }

    query_builder.push(" RETURNING *");

    Ok(query_builder)
}

pub fn room_get_query<'a>(
    request: &'a RoomGetRequest,
    access_token: &'a AccessToken,
) -> Result<QueryBuilder<'a, sqlx::Postgres>, Status> {
    let mut query_builder = QueryBuilder::<sqlx::Postgres>::default();

    query_builder.push("SELECT * FROM room WHERE organization_id = ");
    query_builder.push_bind(access_token.organization_id);

    if !request.name.is_empty() {
        if request.name.len() > 100 {
            return Err(Status::invalid_argument("too many names provided, max 100"));
        }

        if let Some(idx) = request
            .name
            .iter()
            .position(|s| !video_database::name::validate(s))
        {
            return Err(Status::invalid_argument(format!(
                "invalid name provided at index {}, names must match {}",
                idx,
                video_database::name::REGEX
            )));
        }

        let mut names = request
            .name
            .iter()
            .map(|name| name.to_lowercase())
            .collect::<Vec<_>>();
        names.sort();
        names.dedup();

        query_builder.push(" AND name = ANY(");
        query_builder.push_bind(names);
        query_builder.push("::text[])");
    }

    if let Some(transcoding_config_name) = &request.transcoding_config_name {
        if transcoding_config_name.is_empty() {
            query_builder.push(" AND transcoding_config_name IS NULL");
        } else {
            if !video_database::name::validate(transcoding_config_name) {
                return Err(Status::invalid_argument(format!(
                    "invalid transcoding_config_name provided, names must match {}",
                    video_database::name::REGEX
                )));
            }

            query_builder.push(" AND transcoding_config_name = ");
            query_builder.push_bind(transcoding_config_name);
        }
    }

    if let Some(recording_config_name) = &request.recording_config_name {
        if recording_config_name.is_empty() {
            query_builder.push(" AND recording_config_name IS NULL");
        } else {
            if !video_database::name::validate(recording_config_name) {
                return Err(Status::invalid_argument(format!(
                    "invalid recording_config_name provided, names must match {}",
                    video_database::name::REGEX
                )));
            }

            query_builder.push(" AND recording_config_name = ");
            query_builder.push_bind(recording_config_name);
        }
    }

    if let Some(private) = request.private {
        query_builder.push(" AND private = ");
        query_builder.push_bind(private);
    }

    if let Some(live) = request.live {
        query_builder.push(" AND live = ");
        query_builder.push_bind(live);
    }

    // Used to filter out rooms that were created before a certain time
    if let Some(created_at) = request.created_at {
        if created_at > Utc::now().timestamp_micros() {
            return Err(Status::invalid_argument(
                "invalid created_at must be in the past",
            ));
        }

        if created_at < 0 {
            return Err(Status::invalid_argument(
                "invalid created_at must be positive",
            ));
        }

        query_builder.push(" AND created_at > ");
        query_builder.push_bind(created_at);
    }

    let mut limit = request.limit;
    if limit == 0 {
        limit = 100;
    } else if limit > 1000 {
        return Err(Status::invalid_argument(
            "limit too large, must be between 1 and 1000",
        ));
    } else if limit < 0 {
        return Err(Status::invalid_argument(
            "limit too small, must be between 1 and 1000",
        ));
    }

    query_builder.push(" ORDER BY created_at LIMIT ");
    query_builder.push_bind(limit);

    Ok(query_builder)
}

pub fn room_reset_key_query<'a>(
    request: &'a RoomResetKeyRequest,
    access_token: &'a AccessToken,
) -> Result<QueryBuilder<'a, sqlx::Postgres>, Status> {
    let mut query_builder = QueryBuilder::<sqlx::Postgres>::default();

    if request.names.len() > 100 {
        return Err(Status::invalid_argument("too many names provided, max 100"));
    }

    if request.names.is_empty() {
        return Err(Status::invalid_argument("no names provided"));
    }

    if let Some(idx) = request
        .names
        .iter()
        .position(|s| !video_database::name::validate(s))
    {
        return Err(Status::invalid_argument(format!(
            "invalid room name provided at index {}, names must match {}",
            idx,
            video_database::name::REGEX
        )));
    }

    let mut names = request
        .names
        .iter()
        .map(|name| name.to_lowercase())
        .collect::<Vec<_>>();

    names.sort();
    names.dedup();

    query_builder
        .push("UPDATE room as r SET stream_key = v.stream_key, updated_at = NOW() FROM (VALUES ");

    let mut separated = query_builder.separated(", ");

    names.into_iter().for_each(|name| {
        separated.push("(");
        separated.push_bind_unseparated(name);
        separated.push_unseparated(", ");
        separated.push_bind_unseparated(generate_stream_key());
        separated.push_unseparated(")");
    });

    query_builder.push(") AS v(name, stream_key) WHERE r.organization_id = ");
    query_builder.push_bind(access_token.organization_id);
    query_builder.push(" AND r.name = v.name RETURNING r.*");

    Ok(query_builder)
}
