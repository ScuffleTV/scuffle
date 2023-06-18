use std::net::SocketAddr;

use anyhow::Result;
use common::config::{LoggingConfig, RedisConfig, TlsConfig};

#[derive(Debug, Clone, PartialEq, config::Config, serde::Deserialize)]
#[serde(default)]
pub struct EdgeConfig {
    /// Bind Address
    pub bind_address: SocketAddr,

    /// If we should use TLS
    pub tls: Option<TlsConfig>,
}

impl Default for EdgeConfig {
    fn default() -> Self {
        Self {
            bind_address: "[::]:9080".to_string().parse().unwrap(),
            tls: None,
        }
    }
}

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
            bind_address: "[::]:50055".to_string().parse().unwrap(),
            tls: None,
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

    /// API client configuration
    pub edge: EdgeConfig,

    /// gRPC server configuration
    pub grpc: GrpcConfig,

    /// Redis configuration
    pub redis: RedisConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            name: "scuffle-transcoder".to_string(),
            config_file: Some("config".to_string()),
            edge: EdgeConfig::default(),
            grpc: GrpcConfig::default(),
            logging: LoggingConfig::default(),
            redis: RedisConfig::default(),
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
