pub const ACTIVE_EXPIRE_SECONDS: i64 = 450;
pub const INACTIVE_EXPIRE_SECONDS: i64 = 4;
pub const ACTIVE_SEGMENT_COUNT: u32 = 4; // segments
pub const ACTIVE_FRAGMENT_SEGMENT_COUNT: u32 = 2; // segments
pub const FRAGMENT_CUT_TARGET_DURATION: f64 = 0.25; // seconds
pub const FRAGMENT_CUT_MAX_DURATION: f64 = 0.35; // seconds
pub const SEGMENT_CUT_TARGET_DURATION: f64 = 2.0; // seconds

#[inline(always)]
pub fn redis_init_key(stream_id: &str, variant_id: &str) -> String {
    format!("transcoder:{}:{}:init", stream_id, variant_id)
}

#[inline(always)]
pub fn redis_mutex_key(stream_id: &str, variant_id: &str) -> String {
    format!("transcoder:{}:{}:mutex", stream_id, variant_id)
}

#[inline(always)]
pub fn redis_state_key(stream_id: &str, variant_id: &str) -> String {
    format!("transcoder:{}:{}:state", stream_id, variant_id)
}

#[inline(always)]
pub fn redis_segment_state_key(stream_id: &str, variant_id: &str, segment_idx: u32) -> String {
    format!(
        "transcoder:{}:{}:{}:state",
        stream_id, variant_id, segment_idx
    )
}

#[inline(always)]
pub fn redis_segment_data_key(stream_id: &str, variant_id: &str, segment_idx: u32) -> String {
    format!(
        "transcoder:{}:{}:{}:data",
        stream_id, variant_id, segment_idx
    )
}
