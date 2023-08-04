use ulid::Ulid;
use video_database::rendition::Rendition;

pub fn part(
    organization_id: Ulid,
    room_id: Ulid,
    connection_id: Ulid,
    rendition: Rendition,
    part_idx: u32,
) -> String {
    format!(
        "{organization_id}.{room_id}.{connection_id}.part.{rendition}.{part_idx}",
    )
}

pub fn rendition_manifest(
    organization_id: Ulid,
    room_id: Ulid,
    connection_id: Ulid,
    rendition: Rendition,
) -> String {
    format!(
        "{organization_id}.{room_id}.{connection_id}.manifest.{rendition}",
    )
}

pub fn manifest(
    organization_id: Ulid,
    room_id: Ulid,
    connection_id: Ulid,
) -> String {
    format!(
        "{organization_id}.{room_id}.{connection_id}.manifest",
    )
}

pub fn init(
    organization_id: Ulid,
    room_id: Ulid,
    connection_id: Ulid,
    rendition: Rendition,
) -> String {
    format!(
        "{organization_id}.{room_id}.{connection_id}.init.{rendition}",
    )
}

pub fn screenshot(organization_id: Ulid, room_id: Ulid, connection_id: Ulid, idx: u32) -> String {
    format!(
        "{organization_id}.{room_id}.{connection_id}.screenshot.{idx}",
    )
}
