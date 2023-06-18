use std::net::SocketAddr;

use anyhow::Result;
use common::config::{LoggingConfig, RmqConfig, TlsConfig};

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
pub struct ApiConfig {
    /// The bind address for the API server
    pub addresses: Vec<String>,

    /// Resolve interval in seconds (0 to disable)
    pub resolve_interval: u64,

    /// If we should use TLS for the API server
    pub tls: Option<TlsConfig>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            addresses: vec!["localhost:50051".to_string()],
            resolve_interval: 30, // 30 seconds
            tls: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, config::Config, serde::Deserialize)]
#[serde(default)]
pub struct TranscoderConfig {
    pub events_subject: String,
}

impl Default for TranscoderConfig {
    fn default() -> Self {
        Self {
            events_subject: "transcoder".to_string(),
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

    /// API client configuration
    pub api: ApiConfig,

    /// RMQ configuration
    pub rmq: RmqConfig,

    /// Transcoder configuration
    pub transcoder: TranscoderConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            name: "scuffle-ingest".to_string(),
            config_file: Some("config".to_string()),
            logging: LoggingConfig::default(),
            rtmp: RtmpConfig::default(),
            grpc: GrpcConfig::default(),
            api: ApiConfig::default(),
            rmq: RmqConfig::default(),
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
