use std::net::SocketAddr;

use anyhow::Result;
use common::config::{LoggingConfig, RedisConfig, TlsConfig};

#[derive(Debug, Clone, PartialEq, config::Config, serde::Deserialize)]
#[serde(default)]
/// The API is the backend for the Scuffle service
pub struct AppConfig {
    /// The path to the config file
    pub config_file: Option<String>,

    /// Name of this instance
    pub name: String,

    ///  The logging config
    pub logging: LoggingConfig,

    /// Database Config
    pub database: DatabaseConfig,

    /// GRPC Config
    pub grpc: GrpcConfig,

    /// Redis configuration
    pub redis: RedisConfig,

    /// JWT secret used for access tokens
    pub jwt_secret: String,
}

#[derive(Debug, Clone, PartialEq, config::Config, serde::Deserialize)]
#[serde(default)]
pub struct ApiConfig {
    /// Bind address for the API
    pub bind_address: SocketAddr,

    /// If we should use TLS for the API server
    pub tls: Option<TlsConfig>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            bind_address: "[::]:4000".parse().expect("failed to parse bind address"),
            tls: None,
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
            uri: "postgres://root@localhost:5432/scuffle_dev".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, config::Config, serde::Deserialize)]
#[serde(default)]
pub struct GrpcConfig {
    /// Bind address for the GRPC server
    pub bind_address: SocketAddr,

    /// If we should use TLS for the gRPC server
    pub tls: Option<TlsConfig>,
}

impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            bind_address: "[::]:50051".parse().expect("failed to parse bind address"),
            tls: None,
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            config_file: Some("config".to_string()),
            name: "scuffle-api".to_string(),
            logging: LoggingConfig::default(),
            database: DatabaseConfig::default(),
            grpc: GrpcConfig::default(),
            redis: RedisConfig::default(),
            jwt_secret: "secret".to_string(),
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
