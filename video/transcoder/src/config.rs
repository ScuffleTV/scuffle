use std::net::SocketAddr;

use anyhow::Result;
use common::config::{LoggingConfig, RedisConfig, RmqConfig, TlsConfig};

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

    /// The name of the RMQ queue to use
    pub rmq_queue: String,

    /// The uid to use for the unix socket and ffmpeg process
    pub uid: u32,

    /// The gid to use for the unix socket and ffmpeg process
    pub gid: u32,
}

impl Default for TranscoderConfig {
    fn default() -> Self {
        Self {
            rmq_queue: "transcoder".to_string(),
            socket_dir: format!("/tmp/{}", std::process::id()),
            uid: 1000,
            gid: 1000,
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

    /// RMQ configuration
    pub rmq: RmqConfig,

    /// Redis configuration
    pub redis: RedisConfig,

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
            rmq: RmqConfig::default(),
            redis: RedisConfig::default(),
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
