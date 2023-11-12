use crate::database::Rendition;
use ulid::Ulid;

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

pub fn screenshot(organization_id: Ulid, room_id: Ulid, connection_id: Ulid, idx: u32) -> String {
    format!("{organization_id}.{room_id}.{connection_id}.screenshot.{idx}",)
}

pub fn s3_segment(
    organization_id: Ulid,
    recording_id: Ulid,
    rendition: Rendition,
    segment_idx: u32,
    segment_id: Ulid,
) -> String {
    format!("{organization_id}/{recording_id}/{rendition}/{segment_idx}.{segment_id}.mp4",)
}

pub fn s3_thumbnail(
    organization_id: Ulid,
    recording_id: Ulid,
    thumbnail_idx: u32,
    thumbnail_id: Ulid,
) -> String {
    format!("{organization_id}/{recording_id}/thumbnails/{thumbnail_idx}.{thumbnail_id}.jpg",)
}

pub fn s3_init(organization_id: Ulid, recording_id: Ulid, rendition: Rendition) -> String {
    format!("{organization_id}/{recording_id}/{rendition}/init.mp4",)
}