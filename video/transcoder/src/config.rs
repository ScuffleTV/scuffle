use std::time::Duration;

use common::config::TlsConfig;

#[derive(Debug, Clone, PartialEq, config::Config, serde::Deserialize)]
#[serde(default)]
pub struct TranscoderConfig {
	/// The direcory to create unix sockets in
	pub socket_dir: String,

	/// The name of the transcoder requests queue to use
	pub transcoder_request_subject: String,

	/// The uid to use for the unix socket and ffmpeg process
	pub ffmpeg_uid: u32,

	/// The gid to use for the unix socket and ffmpeg process
	pub ffmpeg_gid: u32,

	/// The NATS KV bucket to use for metadata
	pub metadata_kv_store: String,

	/// The NATS ObjectStore bucket to use for media
	pub media_ob_store: String,

	/// The target segment length
	pub min_segment_duration: Duration,

	/// The target part length
	pub target_part_duration: Duration,

	/// The maximum part length
	pub max_part_duration: Duration,

	/// The TLS config to use when connecting to ingest
	pub ingest_tls: Option<TlsConfig>,

	/// The number of segments to keep in the playlist
	pub playlist_segments: usize,
}

impl Default for TranscoderConfig {
	fn default() -> Self {
		Self {
			transcoder_request_subject: "transcoder-request".to_string(),
			socket_dir: format!("/tmp/{}", std::process::id()),
			ffmpeg_uid: 1000,
			ffmpeg_gid: 1000,
			metadata_kv_store: "transcoder-metadata".to_string(),
			media_ob_store: "transcoder-media".to_string(),
			min_segment_duration: Duration::from_secs(2),
			target_part_duration: Duration::from_millis(250),
			max_part_duration: Duration::from_millis(500),
			ingest_tls: None,
			playlist_segments: 5,
		}
	}
}
