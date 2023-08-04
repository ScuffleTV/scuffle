use std::{net::SocketAddr, time::Duration};

use anyhow::Result;
use common::config::{LoggingConfig, NatsConfig, TlsConfig};

#[derive(Debug, Clone, PartialEq, config::Config, serde::Deserialize)]
#[serde(default)]
pub struct GrpcConfig {
    /// The bind address for the gRPC server
    pub bind_address: SocketAddr,

    /// If we should use TLS for the gRPC server
    pub tls: Option<TlsConfig>,
}

impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            bind_address: "[::]:50053".to_string().parse().unwrap(),
            tls: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, config::Config, serde::Deserialize)]
#[serde(default)]
pub struct IngestConfig {
    /// If we should use TLS for the API server
    pub tls: Option<TlsConfig>,
}

#[derive(Debug, Clone, PartialEq, config::Config, serde::Deserialize)]
#[serde(default)]
pub struct TranscoderConfig {
    /// The direcory to create unix sockets in
    pub socket_dir: String,

    /// The name of the transcoder requests queue to use
    pub transcoder_request_subject: String,

    /// The name of the events queue to use
    pub events_subject: String,

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
            events_subject: "events".to_string(),
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

#[derive(Debug, Clone, PartialEq, config::Config, serde::Deserialize)]
#[serde(default)]
pub struct DatabaseConfig {
    /// The database URL to use
    pub uri: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            uri: "postgres://root@localhost:5432/scuffle_video".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, config::Config, serde::Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// Name of this instance
    pub name: String,

    /// The path to the config file.
    pub config_file: Option<String>,

    /// The log level to use, this is a tracing env filter
    pub logging: LoggingConfig,

    /// gRPC server configuration
    pub grpc: GrpcConfig,

    /// NATS configuration
    pub nats: NatsConfig,

    /// Database configuration
    pub database: DatabaseConfig,

    /// Transcoder configuration
    pub transcoder: TranscoderConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            name: "scuffle-transcoder".to_string(),
            config_file: Some("config".to_string()),
            grpc: GrpcConfig::default(),
            logging: LoggingConfig::default(),
            nats: NatsConfig::default(),
            database: DatabaseConfig::default(),
            transcoder: TranscoderConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn parse() -> Result<Self> {
        let (mut config, config_file) =
            common::config::parse::<Self>(!cfg!(test), Self::default().config_file)?;

        config.config_file = config_file;

        Ok(config)
    }
}
