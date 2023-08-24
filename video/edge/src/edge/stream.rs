use std::sync::Arc;
use std::time::Duration;

use chrono::TimeZone;
use common::prelude::FutureTimeout;
use common::vec_of_strings;
use futures_util::StreamExt;
use hmac::{Hmac, Mac};
use hyper::{Body, Request, Response, StatusCode};
use jwt::{AlgorithmType, PKeyWithDigest, SignWithKey, Token, VerifyWithKey};
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use pb::ext::UlidExt;
use pb::scuffle::video::internal::live_rendition_manifest::RenditionInfo;
use pb::scuffle::video::internal::{LiveManifest, LiveRenditionManifest};
use prost::Message;
use routerify::{prelude::RequestExt, Router};
use sha2::Sha256;
use sqlx::Row;
use tokio::io::AsyncReadExt;
use tokio::time::Instant;
use ulid::Ulid;
use uuid::Uuid;
use video_database::playback_key_pair::PlaybackKeyPair;
use video_database::recording::Recording;
use video_database::recording_rendition::RecordingRendition;
use video_database::rendition::Rendition;
use video_database::room::Room;
use video_database::room_status::RoomStatus;

use super::error::{Result, RouteError};
use crate::edge::error::ResultExt;
use crate::{edge::ext::RequestExt as _, global::GlobalState};
mod keys {
    use ulid::Ulid;
    use video_database::rendition::Rendition;

    pub fn part(
        organization_id: Ulid,
        room_id: Ulid,
        connection_id: Ulid,
        rendition: Rendition,
        part_idx: u32,
    ) -> String {
        format!("{organization_id}.{room_id}.{connection_id}.part.{rendition}.{part_idx}",)
    }

    pub fn rendition_manifest(
        organization_id: Ulid,
        room_id: Ulid,
        connection_id: Ulid,
        rendition: Rendition,
    ) -> String {
        format!("{organization_id}.{room_id}.{connection_id}.manifest.{rendition}",)
    }

    pub fn manifest(organization_id: Ulid, room_id: Ulid, connection_id: Ulid) -> String {
        format!("{organization_id}.{room_id}.{connection_id}.manifest",)
    }

    pub fn init(
        organization_id: Ulid,
        room_id: Ulid,
        connection_id: Ulid,
        rendition: Rendition,
    ) -> String {
        format!("{organization_id}.{room_id}.{connection_id}.init.{rendition}",)
    }

    pub fn screenshot(
        organization_id: Ulid,
        room_id: Ulid,
        connection_id: Ulid,
        idx: u32,
    ) -> String {
        format!("{organization_id}.{room_id}.{connection_id}.screenshot.{idx}",)
    }
}

#[derive(Debug, serde::Deserialize)]
struct TokenClaims {
    /// The room name that this token is for (required)
    organization_id: Option<Ulid>,

    /// The room name that this token is for (required)
    room_id: Option<Ulid>,

    /// The time at which the token was issued (required)
    iat: Option<i64>,

    /// Used to create single use tokens (optional)
    id: Option<String>,

    /// The user ID that this token is for (optional)
    user_id: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct MediaClaims {
    organization_id: Ulid,
    room_id: Ulid,
    connection_id: Ulid,
    rendition: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    idx: Vec<u32>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct ScreenshotClaims {
    organization_id: Ulid,
    room_id: Ulid,
    connection_id: Ulid,
    idx: u32,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct SessionClaims {
    id: Ulid,
    organization_id: Ulid,
    connection_id: Ulid,
    room_id: Ulid,
    iat: i64,
    was_authenticated: bool,
}

// https://edge.scuffle.tv/<organization_id>/<room_name>/index.m3u8?token=<token>
async fn room_playlist(req: Request<Body>) -> Result<Response<Body>> {
    let global = req.get_global()?;

    let organization_id = Ulid::from_string(req.param("organization_id").unwrap())
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid organization_id"))?;
    let room_id = Ulid::from_string(req.param("room_id").unwrap())
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid room_id"))?;
    let token = req.uri().query().and_then(|v| {
        url::form_urlencoded::parse(v.as_bytes()).find_map(|(k, v)| {
            if k == "token" {
                Some(v.to_string())
            } else {
                None
            }
        })
    });

    let token = if let Some(token) = token {
        let token: Token<jwt::Header, TokenClaims, _> = Token::parse_unverified(&token)
            .map_err(|_| (StatusCode::BAD_REQUEST, "invalid token, could not parse"))?;

        let playback_key_pair_id = Ulid::from_string(
            token
                .header()
                .key_id
                .as_ref()
                .ok_or((StatusCode::BAD_REQUEST, "invalid token, missing key id"))?,
        )
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid token, invalid key id"))?;

        if token.header().algorithm != AlgorithmType::Es384 {
            return Err((
                StatusCode::BAD_REQUEST,
                "invalid token, invalid algorithm, only ES384 is supported",
            )
                .into());
        }

        if &organization_id
            != token.claims().organization_id.as_ref().ok_or((
                StatusCode::BAD_REQUEST,
                "invalid token, missing organization id",
            ))?
        {
            return Err((
                StatusCode::BAD_REQUEST,
                "invalid token, organization id mismatch",
            )
                .into());
        }

        if &room_id
            != token
                .claims()
                .room_id
                .as_ref()
                .ok_or((StatusCode::BAD_REQUEST, "invalid token, missing room id"))?
        {
            return Err((StatusCode::BAD_REQUEST, "invalid token, room id mismatch").into());
        }

        let iat = token
            .claims()
            .iat
            .ok_or((StatusCode::BAD_REQUEST, "invalid token, missing iat"))?;

        if iat > chrono::Utc::now().timestamp() {
            return Err((
                StatusCode::BAD_REQUEST,
                "invalid token, iat is in the future",
            )
                .into());
        }

        if iat < (chrono::Utc::now().timestamp()) - 60 {
            return Err((
                StatusCode::BAD_REQUEST,
                "invalid token, iat is too far in the past",
            )
                .into());
        }

        todo!("check the database if the token has been revoked for this userid or id within the last 60 seconds");

        let keypair: Option<PlaybackKeyPair> = sqlx::query_as(
            "SELECT * FROM playback_key_pairs WHERE organization_id = $1 AND id = $2",
        )
        .bind(Uuid::from(organization_id))
        .bind(Uuid::from(playback_key_pair_id))
        .fetch_optional(global.db.as_ref())
        .await
        .map_err_route((
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to query database",
        ))?;

        let keypair =
            keypair.ok_or((StatusCode::BAD_REQUEST, "invalid token, keypair not found"))?;

        let signing_algo = PKeyWithDigest {
            digest: MessageDigest::sha384(),
            key: PKey::public_key_from_pem(&keypair.public_key).map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to parse public key",
                )
            })?,
        };

        Some(
            token
                .verify_with_key(&signing_algo)
                .map_err(|_| (StatusCode::BAD_REQUEST, "invalid token, failed to verify"))?,
        )
    } else {
        None
    };

    let room: Option<Room> = sqlx::query_as(
        "SELECT * FROM rooms WHERE organization_id = $1 AND id = $2 AND status != $3",
    )
    .bind(Uuid::from(organization_id))
    .bind(Uuid::from(room_id))
    .bind(RoomStatus::Offline)
    .fetch_optional(global.db.as_ref())
    .await
    .map_err_route((
        StatusCode::INTERNAL_SERVER_ERROR,
        "failed to query database",
    ))?;

    let room = room.ok_or((StatusCode::NOT_FOUND, "room not found"))?;

    let connection_id = Ulid::from(
        room.active_ingest_connection_id
            .ok_or((StatusCode::NOT_FOUND, "room not found"))?,
    );

    let audio_output = room
        .audio_output
        .ok_or((StatusCode::NOT_FOUND, "room not found"))?;

    let video_output = room
        .video_output
        .ok_or((StatusCode::NOT_FOUND, "room not found"))?;

    if room.private && token.is_none() {
        return Err((
            StatusCode::UNAUTHORIZED,
            "room is private, token is required",
        )
            .into());
    }

    let id = Ulid::new();

    sqlx::query(
        r#"
    INSERT INTO playback_sessions (
        id,
        organization_id,
        room_id,
        user_id,
        playback_key_pair_id,
        issued_at,
        ip_address,
        user_agent,
        referer,
        origin,
        player_version
    ) VALUES (
        $1,
        $2,
        $3,
        $4,
        $5,
        $6,
        $7,
        $8,
        $9,
        $10,
        $11
    )
    "#,
    )
    .bind(Uuid::from(id))
    .bind(Uuid::from(organization_id))
    .bind(Uuid::from(room_id))
    .bind(token.as_ref().and_then(|t| t.claims().user_id.as_ref()))
    .bind(token.as_ref().and_then(|t| t.header().key_id.as_ref()))
    .bind(token.as_ref().and_then(|t| {
        chrono::Utc
            .timestamp_opt(t.claims().iat.unwrap(), 0)
            .single()
    }))
    .bind(req.remote_addr().ip().to_string())
    .bind(
        req.headers()
            .get("user-agent")
            .map(|v| v.to_str().unwrap_or_default()),
    )
    .bind(
        req.headers()
            .get("referer")
            .map(|v| v.to_str().unwrap_or_default()),
    )
    .bind(
        req.headers()
            .get("origin")
            .map(|v| v.to_str().unwrap_or_default()),
    )
    .bind(
        req.headers()
            .get("x-player-version")
            .map(|v| v.to_str().unwrap_or_default()),
    )
    .execute(global.db.as_ref())
    .await
    .map_err_route((
        StatusCode::INTERNAL_SERVER_ERROR,
        "failed to create session",
    ))?;

    let claims = SessionClaims {
        id,
        organization_id,
        connection_id,
        room_id,
        was_authenticated: token.is_some(),
        iat: chrono::Utc::now().timestamp(),
    };

    let key: Hmac<Sha256> = Hmac::new_from_slice(global.config.edge.session_key.as_bytes())
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to create session key",
            )
        })?;
    let session = claims
        .sign_with_key(&key)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "failed to sign session"))?;

    #[rustfmt::skip]
    let mut manifest = vec_of_strings![
        "#EXTM3U",
        "#EXT-X-INDEPENDENT-SEGMENTS",
    ];

    let audio = audio_output.first().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "no audio rendition found",
    ))?;

    let audio_rendition = Rendition::from(audio.rendition());

    manifest.push(format!("#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"audio\",NAME=\"{audio_rendition}\",DEFAULT=YES,AUTOSELECT=YES,URI=\"/{organization_id}/{room_id}/{session}/{audio_rendition}.m3u8\"", ));

    for video in &video_output {
        let video_rendition = Rendition::from(video.rendition());
        manifest.push(format!("#EXT-X-STREAM-INF:BANDWIDTH={},CODECS=\"{},{}\",RESOLUTION={}x{},FRAME-RATE={},AUDIO=\"audio\"", audio.bitrate + video.bitrate, video.codec, audio.codec, video.width, video.height, video.fps));
        manifest.push(format!(
            "/{organization_id}/{room_id}/{session}/{video_rendition}.m3u8"
        ));
    }

    let manifest = manifest.join("\n");
    let mut resp = Response::new(Body::from(manifest));
    resp.headers_mut().insert(
        "Content-Type",
        "application/vnd.apple.mpegurl".parse().unwrap(),
    );
    resp.headers_mut()
        .insert("Cache-Control", "no-cache".parse().unwrap());

    Ok(resp)
}

async fn session_playlist(req: Request<Body>) -> Result<Response<Body>> {
    let global = req.get_global()?;

    let organization_id = Ulid::from_string(req.param("organization_id").unwrap())
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid organization_id"))?;

    let session = req.param("session").unwrap();

    let room_id = Ulid::from_string(req.param("room_id").unwrap())
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid room_id"))?;

    let rendition: Rendition = req
        .param("rendition")
        .unwrap()
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid variant_id"))?;

    let session_key: Hmac<Sha256> = Hmac::new_from_slice(global.config.edge.session_key.as_bytes())
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to create session key",
            )
        })?;

    let session: SessionClaims = session
        .verify_with_key(&session_key)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid session"))?;

    if session.organization_id != organization_id {
        return Err((
            StatusCode::BAD_REQUEST,
            "invalid session, organization_id mismatch",
        )
            .into());
    }

    if session.room_id != room_id {
        return Err((StatusCode::BAD_REQUEST, "invalid session, room_id mismatch").into());
    }

    let resp = sqlx::query(
        r#"
    UPDATE playback_sessions SET
        expires_at = NOW() + INTERVAL '10 minutes'
    WHERE
        id = $1 AND
        organization_id = $2 AND
        room_id = $3 AND
        expires_at > NOW()
    "#,
    )
    .bind(Uuid::from(session.id))
    .bind(Uuid::from(session.organization_id))
    .bind(Uuid::from(session.room_id))
    .execute(global.db.as_ref())
    .await
    .map_err_route((
        StatusCode::INTERNAL_SERVER_ERROR,
        "failed to update session",
    ))?;

    if resp.rows_affected() == 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            "invalid session, expired or not found",
        )
            .into());
    }

    #[derive(Default, Debug)]
    struct HlsConfig {
        msn: Option<u32>,
        part: Option<u32>,
        scuffle_part: Option<u32>,
        skip: bool,
        scuffle_dvr: bool,
    }

    let hls_config = req
        .uri()
        .query()
        .map(|v| {
            url::form_urlencoded::parse(v.as_bytes()).fold(
                HlsConfig::default(),
                |mut acc, (key, value)| {
                    match key.as_ref() {
                        "_HLS_msn" => {
                            acc.msn = value.parse::<u32>().ok();
                        }
                        "_HLS_part" => {
                            acc.part = value.parse::<u32>().ok();
                        }
                        "_HLS_skip" => {
                            acc.skip = value == "YES" || value == "v2";
                        }
                        "_SCUFFLE_PART" => {
                            acc.scuffle_part = value.parse::<u32>().ok();
                        }
                        "_SCUFFLE_DVR" => {
                            acc.scuffle_dvr = value.parse::<bool>().unwrap_or_default();
                        }
                        _ => {}
                    }

                    acc
                },
            )
        })
        .unwrap_or_else(HlsConfig::default);

    let manifest = global
        .metadata_store
        .get(keys::rendition_manifest(
            organization_id,
            room_id,
            session.connection_id,
            rendition,
        ))
        .await
        .map_err_route((StatusCode::INTERNAL_SERVER_ERROR, "failed to get manifest"))?
        .ok_or((StatusCode::NOT_FOUND, "manifest not found"))?;

    let mut manifest = LiveRenditionManifest::decode(manifest).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to decode manifest",
        )
    })?;

    enum BlockStyle {
        Hls(u32, u32),
        Scuffle(u32),
    }

    impl BlockStyle {
        fn is_blocked(&self, info: &RenditionInfo) -> bool {
            let segment_idx = info.next_segment_idx.saturating_sub(1);
            let part_idx = info.next_part_idx.saturating_sub(1);
            let segment_part_idx = info.next_segment_part_idx.saturating_sub(1);

            match self {
                BlockStyle::Hls(hls_msn, hls_part) => {
                    segment_idx < *hls_msn
                        || (segment_idx == *hls_msn && segment_part_idx < *hls_part)
                }
                BlockStyle::Scuffle(scuffle_part) => part_idx < *scuffle_part,
            }
        }
    }

    let block_style = match (hls_config.msn, hls_config.part, hls_config.scuffle_part) {
        (Some(msn), p, None) => Some(BlockStyle::Hls(msn, p.unwrap_or(0))),
        (None, None, Some(p)) => Some(BlockStyle::Scuffle(p)),
        (None, None, None) => None,
        _ => return Err((StatusCode::BAD_REQUEST, "invalid query params").into()),
    };

    if let Some(block_style) = block_style {
        let info = manifest
            .info
            .as_ref()
            .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "manifest missing info"))?;

        if !manifest.completed && block_style.is_blocked(info) {
            // We need to block and wait for the next segment to be available
            // before we can serve this request.
            let mut watch_manifest = global
                .metadata_store
                .watch(keys::rendition_manifest(
                    organization_id,
                    room_id,
                    session.connection_id,
                    rendition,
                ))
                .await
                .map_err_route((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to watch manifest",
                ))?;

            let now = Instant::now();
            loop {
                let entry = watch_manifest
                    .next()
                    .timeout(Duration::from_secs(2))
                    .await
                    .map_err_route((StatusCode::BAD_REQUEST, "segment watch time timedout"))?
                    .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "manifest stream closed"))?
                    .map_err_route((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "failed to watch manifest",
                    ))?;

                manifest = LiveRenditionManifest::decode(entry.value).map_err(|_| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "failed to decode manifest",
                    )
                })?;

                let info = manifest
                    .info
                    .as_ref()
                    .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "manifest missing info"))?;

                if manifest.completed || !block_style.is_blocked(info) {
                    break;
                }

                if now.elapsed() > Duration::from_secs(3) {
                    return Err((StatusCode::BAD_REQUEST, "segment watch time timedout").into());
                }
            }
        }
    }

    let info = manifest
        .info
        .as_ref()
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "manifest missing info"))?;

    const MAX_SEGMENT_DURATION: u32 = 5;
    const TARGET_PART_DURATION: f64 = 0.25;
    const PART_HOLD_BACK: f64 = 3.0 * TARGET_PART_DURATION;

    let mut media_sequence = manifest.segments.first().map(|s| s.idx).unwrap_or_default();

    let media_key: Hmac<Sha256> = Hmac::new_from_slice(global.config.edge.media_key.as_bytes())
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to create media key",
            )
        })?;

    let init_jwt = MediaClaims {
        connection_id: session.connection_id,
        idx: vec![],
        organization_id,
        rendition: rendition.to_string(),
        room_id: session.room_id,
    }
    .sign_with_key(&media_key)
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "failed to sign init"))?;

    let mut server_control = vec_of_strings![
        format!("PART-HOLD-BACK={PART_HOLD_BACK:.3}"),
        "CAN-BLOCK-RELOAD=YES",
    ];

    let mut version = 6;

    let recording: Option<Recording> = if let Some(id) = manifest.recording_ulid {
        sqlx::query_as(
            r#"
            SELECT
                * 
            FROM recordings
            WHERE 
                id = $1 
                AND organization_id = $2
                AND room_id = $3
                AND deleted = FALSE
                AND allow_dvr = TRUE
        "#,
        )
        .bind(Uuid::from(id.to_ulid()))
        .bind(Uuid::from(organization_id))
        .bind(Uuid::from(room_id))
        .fetch_optional(global.db.as_ref())
        .await
        .map_err_route((
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to query database",
        ))?
    } else {
        None
    };

    let can_dvr = if let Some(recording) = &recording {
        recording.public || session.was_authenticated
    } else {
        false
    } && hls_config.scuffle_dvr;

    if can_dvr {
        server_control.push(format!("CAN-SKIP-UNTIL={:.3}", MAX_SEGMENT_DURATION * 3));
        version = 9;
        media_sequence = 0;
    }

    let mut playlist = vec_of_strings![
        "#EXTM3U",
        format!("#EXT-X-VERSION:{}", version),
        format!("#EXT-X-TARGETDURATION:{MAX_SEGMENT_DURATION}"),
        format!("#EXT-X-MEDIA-SEQUENCE:{media_sequence}"),
        format!("#EXT-X-DISCONTINUITY-SEQUENCE:0"),
        format!("#EXT-X-PART-INF:PART-TARGET={TARGET_PART_DURATION:.3}"),
        format!("#EXT-X-SERVER-CONTROL:{}", server_control.join(",")),
        format!("#EXT-X-MAP:URI=\"/{organization_id}/{room_id}/{init_jwt}.mp4\""),
    ];

    let public_s3_url = if can_dvr && !hls_config.skip {
        let recording_id = recording.as_ref().unwrap().id;

        let recording_rendition: RecordingRendition = sqlx::query_as(
            r#"
            SELECT
                r.*
                s.public_url as public_url
            FROM recording_renditions AS r
            WHERE 
                recording_id = $1
                AND rendition = $2
            INNER JOIN s3_buckets s 
            ON s.id = r.s3_bucket_id
            "#,
        )
        .bind(recording_id)
        .bind(rendition)
        .fetch_one(global.db.as_ref())
        .await
        .map_err_route((
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to query database",
        ))?;

        for (idx, (id, duration)) in recording_rendition
            .segment_ids
            .iter()
            .zip(recording_rendition.segment_durations.iter())
            .enumerate()
        {
            let id = Ulid::from(*id);
            let duration = *duration as f64 / recording_rendition.timescale as f64;
            let url = format!(
                "{}/{organization_id}/{recording_id}/{rendition}/{idx}.{id}.mp4",
                recording_rendition.public_url.as_ref().unwrap()
            );
            playlist.push(format!("#EXTINF:{duration:.3},"));
            playlist.push(url);
        }

        recording_rendition.public_url
    } else if can_dvr && hls_config.skip {
        playlist.push(format!(
            "#EXT-X-SKIP:SKIPPED-SEGMENTS={}",
            manifest.segments.first().map(|s| s.idx).unwrap_or_default()
        ));

        let recording_id = recording.as_ref().unwrap().id;

        let row = sqlx::query(
            r#"
            SELECT
                s.public_url as public_url
            FROM recording_renditions AS r
            WHERE 
                recording_id = $1
                AND rendition = $2
            INNER JOIN s3_buckets s 
            ON s.id = r.s3_bucket_id
            "#,
        )
        .bind(recording_id)
        .bind(rendition)
        .fetch_one(global.db.as_ref())
        .await
        .map_err_route((
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to query database",
        ))?;

        row.try_get::<String, _>("public_url").ok()
    } else {
        None
    };

    for segment in &manifest.segments {
        if segment.idx >= info.next_segment_idx.saturating_sub(2) {
            for part in &segment.parts {
                let part_jwt = MediaClaims {
                    connection_id: session.connection_id,
                    idx: vec![part.idx],
                    organization_id,
                    rendition: rendition.to_string(),
                    room_id: session.room_id,
                }
                .sign_with_key(&media_key)
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "failed to sign part"))?;

                let duration = part.duration as f64 / manifest.timescale as f64;

                let independent = if part.independent {
                    ",INDEPENDENT=YES"
                } else {
                    ""
                };

                playlist.push(format!("#EXT-X-PART:DURATION={duration:.3},URI=\"/{organization_id}/{room_id}/{part_jwt}.mp4\"{independent}"));
            }
        }

        if segment.idx != info.next_segment_idx.saturating_sub(1) || manifest.completed {
            let segment_jwt = MediaClaims {
                connection_id: session.connection_id,
                idx: segment.parts.iter().map(|p| p.idx).collect(),
                organization_id,
                rendition: rendition.to_string(),
                room_id: session.room_id,
            }
            .sign_with_key(&media_key)
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "failed to sign segment"))?;

            let duration = segment.parts.iter().map(|p| p.duration).sum::<u32>() as f64
                / manifest.timescale as f64;
            if can_dvr {
                let public_s3_url = public_s3_url.as_ref().unwrap();
                let recording_id = recording.as_ref().unwrap().id;
                let id = segment.id.to_ulid();
                let idx = segment.idx;
                playlist.push(format!("#EXT-X-SCUFFLE-DVR:URI=\"{public_s3_url}/{organization_id}/{recording_id}/{rendition}/{idx}.{id}.mp4\""))
            }
            playlist.push(format!("#EXTINF:{duration:.3},", duration = duration));
            playlist.push(format!("/{organization_id}/{room_id}/{segment_jwt}.mp4"));
        }
    }

    if !manifest.completed {
        for i in 0..5 {
            let part_idx = info.next_part_idx + i;

            let part_jwt = MediaClaims {
                connection_id: session.connection_id,
                idx: vec![part_idx],
                organization_id,
                rendition: rendition.to_string(),
                room_id: session.room_id,
            }
            .sign_with_key(&media_key)
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "failed to sign part"))?;

            playlist.push(format!("#EXT-X-PRELOAD-HINT:TYPE=PART,SCUFFLE-PART={part_idx},URI=\"/{organization_id}/{room_id}/{part_jwt}.mp4\""));
        }

        for (rendition, info) in manifest.other_info {
            let last_msn = info.next_segment_idx.saturating_sub(1);
            let last_part = info.next_segment_part_idx.saturating_sub(1);
            playlist.push(format!("#EXT-X-RENDITION-REPORT:URI=\"./{rendition}.m3u8\",LAST-MSN={last_msn},LAST-PART={last_part}"));
        }
    } else {
        playlist.push("#EXT-X-ENDLIST".to_string());
    }

    let playlist = playlist.join("\n");
    let mut resp = Response::new(Body::from(playlist));
    resp.headers_mut().insert(
        "Content-Type",
        "application/vnd.apple.mpegurl".parse().unwrap(),
    );
    resp.headers_mut()
        .insert("Cache-Control", "no-cache".parse().unwrap());

    Ok(resp)
}

async fn room_media(req: Request<Body>) -> Result<Response<Body>> {
    let global = req.get_global()?;

    let organization_id = Ulid::from_string(req.param("organization_id").unwrap())
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid organization_id"))?;

    let room_id = Ulid::from_string(req.param("room_id").unwrap())
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid room_id"))?;

    let media = req.param("media").unwrap();

    let key: Hmac<Sha256> =
        Hmac::new_from_slice(global.config.edge.media_key.as_bytes()).map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to create media key",
            )
        })?;

    let claims: MediaClaims = media
        .verify_with_key(&key)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid media"))?;

    if claims.organization_id != organization_id {
        return Err((
            StatusCode::BAD_REQUEST,
            "invalid media, organization_id mismatch",
        )
            .into());
    }

    if claims.room_id != room_id {
        return Err((StatusCode::BAD_REQUEST, "invalid media, room_name mismatch").into());
    }

    let keys = match claims.idx.len() {
        0 => vec![keys::init(
            organization_id,
            room_id,
            claims.connection_id,
            claims.rendition.parse().unwrap(),
        )],
        _ => claims
            .idx
            .iter()
            .map(|idx| {
                keys::part(
                    organization_id,
                    room_id,
                    claims.connection_id,
                    claims.rendition.parse().unwrap(),
                    *idx,
                )
            })
            .collect::<Vec<_>>(),
    };

    // Streaming response
    let mut data = Vec::new();

    for key in keys {
        let mut item = global
            .media_store
            .get(&key)
            .await
            .map_err_route((StatusCode::INTERNAL_SERVER_ERROR, "failed to get media"))?;

        item.read_to_end(&mut data)
            .await
            .map_err_route((StatusCode::INTERNAL_SERVER_ERROR, "failed to read media"))?;
    }

    let mut resp = Response::new(Body::from(data));
    resp.headers_mut()
        .insert("Content-Type", "video/mp4".parse().unwrap());
    resp.headers_mut()
        .insert("Cache-Control", "max-age=31536000".parse().unwrap());

    Ok(resp)
}

async fn room_screenshot(req: Request<Body>) -> Result<Response<Body>> {
    let global = req.get_global()?;

    let organization_id = Ulid::from_string(req.param("organization_id").unwrap())
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid organization_id"))?;
    let room_id = Ulid::from_string(req.param("room_id").unwrap())
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid room_id"))?;
    let token = req.uri().query().and_then(|v| {
        url::form_urlencoded::parse(v.as_bytes()).find_map(|(k, v)| {
            if k == "token" {
                Some(v.to_string())
            } else {
                None
            }
        })
    });

    let token = if let Some(token) = token {
        todo!("validate token");

        Some(token)
    } else {
        None
    };

    let room: Option<Room> = sqlx::query_as(
        "SELECT * FROM rooms WHERE organization_id = $1 AND id = $2 AND status != $3",
    )
    .bind(Uuid::from(organization_id))
    .bind(Uuid::from(room_id))
    .bind(RoomStatus::Offline)
    .fetch_optional(global.db.as_ref())
    .await
    .map_err_route((
        StatusCode::INTERNAL_SERVER_ERROR,
        "failed to query database",
    ))?;

    let room = room.ok_or((StatusCode::NOT_FOUND, "room not found"))?;

    let connection_id = Ulid::from(
        room.active_ingest_connection_id
            .ok_or((StatusCode::NOT_FOUND, "room not found"))?,
    );

    if room.private && token.is_none() {
        return Err((
            StatusCode::UNAUTHORIZED,
            "room is private, token is required",
        )
            .into());
    }

    // We have permission to see the screenshot.
    let manifest = global
        .metadata_store
        .get(keys::manifest(organization_id, room_id, connection_id))
        .await
        .map_err_route((StatusCode::INTERNAL_SERVER_ERROR, "failed to get manifest"))?
        .ok_or((StatusCode::NOT_FOUND, "manifest not found"))?;

    let manifest = LiveManifest::decode(manifest).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to decode manifest",
        )
    })?;

    let key: Hmac<Sha256> =
        Hmac::new_from_slice(global.config.edge.media_key.as_bytes()).map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to create media key",
            )
        })?;

    let screenshot = ScreenshotClaims {
        connection_id,
        idx: manifest.screenshot_idx,
        organization_id,
        room_id,
    }
    .sign_with_key(&key)
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to sign screenshot",
        )
    })?;

    let mut response = Response::new(Body::default());

    *response.status_mut() = StatusCode::TEMPORARY_REDIRECT;

    let url = format!("/{organization_id}/{room_id}/{screenshot}.jpg");

    response
        .headers_mut()
        .insert("Location", url.parse().unwrap());

    response
        .headers_mut()
        .insert("Cache-Control", "no-cache".parse().unwrap());

    Ok(response)
}

async fn room_screenshot_media(req: Request<Body>) -> Result<Response<Body>> {
    let global = req.get_global()?;

    let organization_id = Ulid::from_string(req.param("organization_id").unwrap())
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid organization_id"))?;

    let room_id = Ulid::from_string(req.param("room_id").unwrap())
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid room_id"))?;

    let screenshot = req.param("screenshot").unwrap();

    let key: Hmac<Sha256> =
        Hmac::new_from_slice(global.config.edge.media_key.as_bytes()).map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to create media key",
            )
        })?;

    let claims: ScreenshotClaims = screenshot
        .verify_with_key(&key)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid media"))?;

    if claims.organization_id != organization_id {
        return Err((
            StatusCode::BAD_REQUEST,
            "invalid media, organization_id mismatch",
        )
            .into());
    }

    if claims.room_id != room_id {
        return Err((StatusCode::BAD_REQUEST, "invalid media, room_name mismatch").into());
    }

    let key = keys::screenshot(organization_id, room_id, claims.connection_id, claims.idx);

    tracing::debug!(key = %key, "getting screenshot");

    let mut item = global.media_store.get(&key).await.map_err_route((
        StatusCode::INTERNAL_SERVER_ERROR,
        "failed to get screenshot",
    ))?;

    let mut buf = Vec::new();

    item.read_to_end(&mut buf).await.map_err_route((
        StatusCode::INTERNAL_SERVER_ERROR,
        "failed to read screenshot",
    ))?;

    let mut resp = Response::new(Body::from(buf));
    resp.headers_mut()
        .insert("Content-Type", "image/jpeg".parse().unwrap());
    resp.headers_mut()
        .insert("Cache-Control", "max-age=31536000".parse().unwrap());

    Ok(resp)
}

pub fn routes(_: &Arc<GlobalState>) -> Router<Body, RouteError> {
    Router::builder()
        .get("/:organization_id/:room_id.m3u8", room_playlist)
        .get("/:organization_id/:room_id.jpg", room_screenshot)
        .get(
            "/:organization_id/:room_id/:session/:rendition.m3u8",
            session_playlist,
        )
        .get("/:organization_id/:room_id/:media.mp4", room_media)
        .get(
            "/:organization_id/:room_id/:screenshot.jpg",
            room_screenshot_media,
        )
        .build()
        .expect("failed to build router")
}
