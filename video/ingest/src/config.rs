use std::{net::SocketAddr, time::Duration};

use anyhow::Result;
use common::config::{LoggingConfig, NatsConfig, TlsConfig};

#[derive(Debug, Clone, PartialEq, config::Config, serde::Deserialize)]
#[serde(default)]
pub struct RtmpConfig {
    /// The bind address for the RTMP server
    pub bind_address: SocketAddr,

    /// If we should use TLS for the RTMP server
    pub tls: Option<TlsConfig>,
}

impl Default for RtmpConfig {
    fn default() -> Self {
        Self {
            bind_address: "[::]:1935".to_string().parse().unwrap(),
            tls: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, config::Config, serde::Deserialize)]
#[serde(default)]
pub struct GrpcConfig {
    /// The bind address for the gRPC server
    pub bind_address: SocketAddr,

    /// Advertising address for the gRPC server
    pub advertise_address: String,

    /// If we should use TLS for the gRPC server
    pub tls: Option<TlsConfig>,
}

impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            bind_address: "[::]:50052".to_string().parse().unwrap(),
            advertise_address: "".to_string(),
            tls: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, config::Config, serde::Deserialize)]
#[serde(default)]
pub struct IngestConfig {
    // NATS subject to send transcoder requests to
    pub transcoder_request_subject: String,

    /// NATS subject for events
    pub events_subject: String,

    /// The interval in to update the bitrate for a room
    pub bitrate_update_interval: Duration,

    /// The maximum time to wait for a transcoder
    pub transcoder_timeout: Duration,

    /// Max Bitrate for ingest
    pub max_bitrate: u64,

    /// Max bytes between keyframes
    pub max_bytes_between_keyframes: u64,

    /// Max time between keyframes
    pub max_time_between_keyframes: Duration,
}

impl Default for IngestConfig {
    fn default() -> Self {
        Self {
            transcoder_request_subject: "transcoder-request".to_string(),
            events_subject: "events".to_string(),
            bitrate_update_interval: Duration::from_secs(5),
            max_bitrate: 12000 * 1024,
            max_bytes_between_keyframes: 12000 * 1024 * 5 / 8,
            max_time_between_keyframes: Duration::from_secs(10),
            transcoder_timeout: Duration::from_secs(60),
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

    /// RTMP server configuration
    pub rtmp: RtmpConfig,

    /// GRPC server configuration
    pub grpc: GrpcConfig,

    /// Database configuration
    pub database: DatabaseConfig,

    /// NATS configuration
    pub nats: NatsConfig,

    /// Ingest configuration
    pub ingest: IngestConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            name: "scuffle-ingest".to_string(),
            config_file: Some("config".to_string()),
            logging: LoggingConfig::default(),
            rtmp: RtmpConfig::default(),
            grpc: GrpcConfig::default(),
            database: DatabaseConfig::default(),
            nats: NatsConfig::default(),
            ingest: IngestConfig::default(),
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
