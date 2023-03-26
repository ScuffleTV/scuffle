use std::convert::Infallible;
use std::{collections::HashMap, sync::Arc};

use bytes::Bytes;
use futures::stream;
use hyper::{Body, Request, Response, StatusCode};
use routerify::{prelude::RequestExt, Router};

use super::error::{Result, RouteError};
use crate::{edge::ext::RequestExt as _, global::GlobalState};
use fred::interfaces::HashesInterface;
use fred::interfaces::KeysInterface;

pub async fn variant_playlist(req: Request<Body>) -> Result<Response<Body>> {
    let global = req.get_global()?;

    let stream_id = uuid::Uuid::parse_str(req.param("stream_id").unwrap())
        .map_err(|_| (StatusCode::NOT_FOUND, "Not found"))?;
    let variant_id = uuid::Uuid::parse_str(req.param("variant_id").unwrap())
        .map_err(|_| (StatusCode::NOT_FOUND, "Not found"))?;

    tracing::info!(stream_id = ?stream_id, variant_id = ?variant_id, "variant_playlist");

    let params: HashMap<String, String> = req
        .uri()
        .query()
        .map(|v| {
            url::form_urlencoded::parse(v.as_bytes())
                .into_owned()
                .collect()
        })
        .unwrap_or_else(HashMap::new);

    // LL-HLS allows for a few query parameters:
    // - _HLS_msn (Media Sequence Number)
    // - _HLS_part (Part Number)

    // If those are present we should block until the requested sequence number is available.

    let sequence_number = params.get("_HLS_msn").and_then(|v| v.parse::<u64>().ok());
    let part_number = params.get("_HLS_part").and_then(|v| v.parse::<u64>().ok());

    if sequence_number.is_none() && part_number.is_some() {
        return Err((StatusCode::BAD_REQUEST, "Bad Request").into());
    }

    if let Some(sequence_number) = sequence_number {
        let part_number = part_number.unwrap_or_default();

        let mut count = 0;

        loop {
            if count > 10 {
                return Err((StatusCode::BAD_REQUEST, "Bad Request").into());
            }

            let fields: Vec<String> = global
                .redis
                .hmget(
                    &format!("transcoder:{}:{}:state", stream_id, variant_id),
                    vec![
                        "current_segment_idx".to_string(),
                        "current_fragment_idx".to_string(),
                    ],
                )
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Internal Server Error",
                        e,
                    )
                })?;

            let current_segment_idx: u64 = fields[0].parse::<u64>().unwrap_or_default();
            let current_fragment_idx: u64 = fields[1].parse::<u64>().unwrap_or_default();

            tracing::info!(
                sequence_number = sequence_number,
                current_segment_idx = current_segment_idx,
                part_number = part_number,
                current_fragment_idx = current_fragment_idx,
                "waiting for sequence number"
            );

            if sequence_number > current_segment_idx + 3 {
                return Err((StatusCode::BAD_REQUEST, "Bad Request").into());
            }

            if sequence_number < current_segment_idx
                || (sequence_number == current_segment_idx && part_number < current_fragment_idx)
            {
                break;
            }

            count += 1;
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    let playlist: String = global
        .redis
        .hget(
            &format!("transcoder:{}:{}:state", stream_id, variant_id),
            "playlist",
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error",
                e,
            )
        })?;

    if playlist.is_empty() {
        return Err((StatusCode::NOT_FOUND, "Not found").into());
    }

    Ok(Response::builder()
        .header("Content-Type", "application/vnd.apple.mpegurl")
        .header("Cache-Control", "no-cache")
        .body(Body::from(playlist))
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error",
                e,
            )
        })?)
}

pub async fn master_playlist(req: Request<Body>) -> Result<Response<Body>> {
    let global = req.get_global()?;

    let stream_id = uuid::Uuid::parse_str(req.param("stream_id").unwrap())
        .map_err(|_| (StatusCode::NOT_FOUND, "Not found"))?;

    tracing::info!(stream_id = ?stream_id, "master_playlist");

    let playlist: String = global
        .redis
        .get(&format!("transcoder:{}:playlist", stream_id))
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error",
                e,
            )
        })?;

    if playlist.is_empty() {
        return Err((StatusCode::NOT_FOUND, "Not found").into());
    }

    Ok(Response::builder()
        .header("Content-Type", "application/vnd.apple.mpegurl")
        .header("Cache-Control", "no-cache")
        .body(Body::from(playlist))
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error",
                e,
            )
        })?)
}

pub async fn segment(req: Request<Body>) -> Result<Response<Body>> {
    let global = req.get_global()?;

    let stream_id = uuid::Uuid::parse_str(req.param("stream_id").unwrap())
        .map_err(|_| (StatusCode::NOT_FOUND, "Not found"))?;
    let variant_id = uuid::Uuid::parse_str(req.param("variant_id").unwrap())
        .map_err(|_| (StatusCode::NOT_FOUND, "Not found"))?;
    let segment = req.param("segment").unwrap();

    tracing::info!(stream_id = ?stream_id, variant_id = ?variant_id, segment = ?segment, "segment");

    if segment.contains('.') {
        let (segment, part) = segment.split_once('.').unwrap();
        let part_number = part
            .parse::<u64>()
            .map_err(|_| (StatusCode::BAD_REQUEST, "Bad Request"))?;
        let sequence_number = segment
            .parse::<u64>()
            .map_err(|_| (StatusCode::BAD_REQUEST, "Bad Request"))?;

        let mut count = 0;
        loop {
            if count > 10 {
                return Err((StatusCode::BAD_REQUEST, "Bad Request").into());
            }

            let fields: Vec<String> = global
                .redis
                .hmget(
                    &format!("transcoder:{}:{}:state", stream_id, variant_id),
                    vec![
                        "current_segment_idx".to_string(),
                        "current_fragment_idx".to_string(),
                    ],
                )
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Internal Server Error",
                        e,
                    )
                })?;

            let current_segment_idx: u64 = fields[0].parse::<u64>().unwrap_or_default();
            let current_fragment_idx: u64 = fields[1].parse::<u64>().unwrap_or_default();

            tracing::info!(
                sequence_number = sequence_number,
                current_segment_idx = current_segment_idx,
                part_number = part_number,
                current_fragment_idx = current_fragment_idx,
                "waiting for sequence number"
            );

            if sequence_number > current_segment_idx + 3 {
                return Err((StatusCode::BAD_REQUEST, "Bad Request").into());
            }

            if sequence_number < current_segment_idx
                || (sequence_number == current_segment_idx && part_number < current_fragment_idx)
            {
                break;
            }

            count += 1;
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        let part: Option<Bytes> = global
            .redis
            .hget(
                &format!("transcoder:{}:{}:{}:data", stream_id, variant_id, segment),
                part_number.to_string(),
            )
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error",
                    e,
                )
            })?;
        let Some(part) = part else {
            return Err((StatusCode::NOT_FOUND, "Not found").into());
        };

        return Ok(Response::builder()
            .header("Content-Type", "video/mp4")
            .header("Cache-Control", "max-age=31536000")
            .body(Body::from(part))
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error",
                    e,
                )
            })?);
    }

    let state: Vec<String> = global
        .redis
        .hmget(
            &format!("transcoder:{}:{}:{}:state", stream_id, variant_id, segment),
            vec!["ready".to_string(), "fragment_count".to_string()],
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error",
                e,
            )
        })?;
    if state[0] != "true" {
        return Err((StatusCode::NOT_FOUND, "Not found").into());
    }

    let mut data: HashMap<String, Bytes> = global
        .redis
        .hgetall(&format!(
            "transcoder:{}:{}:{}:data",
            stream_id, variant_id, segment
        ))
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error",
                e,
            )
        })?;

    let mut data_vec: Vec<Result<Bytes, Infallible>> = vec![];
    for i in 0..state[1].parse::<u64>().unwrap_or_default() {
        let Some(data) = data.remove(&i.to_string()) else {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into());
        };

        data_vec.push(Ok(data));
    }

    Ok(Response::builder()
        .header("Content-Type", "video/mp4")
        .header("Cache-Control", "max-age=31536000")
        .body(Body::wrap_stream(stream::iter(data_vec)))
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error",
                e,
            )
        })?)
}

pub async fn init_segment(req: Request<Body>) -> Result<Response<Body>> {
    let global = req.get_global()?;

    let stream_id = uuid::Uuid::parse_str(req.param("stream_id").unwrap())
        .map_err(|_| (StatusCode::NOT_FOUND, "Not found"))?;
    let variant_id = uuid::Uuid::parse_str(req.param("variant_id").unwrap())
        .map_err(|_| (StatusCode::NOT_FOUND, "Not found"))?;

    tracing::info!(stream_id = ?stream_id, variant_id = ?variant_id, "init segment");

    let part: Option<Bytes> = global
        .redis
        .get(&format!("transcoder:{}:{}:init", stream_id, variant_id))
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error",
                e,
            )
        })?;
    let Some(part) = part else {
        return Err((StatusCode::NOT_FOUND, "Not found").into());
    };

    Ok(Response::builder()
        .header("Content-Type", "video/mp4")
        .header("Cache-Control", "max-age=31536000")
        .body(Body::from(part))
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error",
                e,
            )
        })?)
}

pub fn routes(_: &Arc<GlobalState>) -> Router<Body, RouteError> {
    Router::builder()
        .get("/:stream_id/:variant_id/index.m3u8", variant_playlist)
        .get("/:stream_id/:variant_id/init.mp4", init_segment)
        .get("/:stream_id/master.m3u8", master_playlist)
        .get("/:stream_id/:variant_id/:segment.mp4", segment)
        .build()
        .expect("failed to build router")
}
