use std::net::SocketAddr;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct TlsConfig {
    /// Domain name to use for TLS
    /// Only used for gRPC TLS connections
    pub domain: Option<String>,

    /// The path to the TLS certificate
    pub cert: String,

    /// The path to the TLS private key
    pub key: String,

    /// The path to the TLS CA certificate
    pub ca_cert: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct RedisConfig {
    /// The address of the Redis server
    pub addresses: Vec<String>,

    /// Number of connections to keep in the pool
    pub pool_size: usize,

    /// The username to use for authentication
    pub username: Option<String>,

    /// The password to use for authentication
    pub password: Option<String>,

    /// The database to use
    pub database: u8,

    /// The TLS configuration
    pub tls: Option<TlsConfig>,

    /// To use Redis Sentinel
    pub sentinel: Option<RedisSentinelConfig>,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            addresses: vec!["localhost:6379".to_string()],
            pool_size: 10,
            username: None,
            password: None,
            database: 0,
            tls: None,
            sentinel: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct RedisSentinelConfig {
    /// The master group name
    pub service_name: String,
}

impl Default for RedisSentinelConfig {
    fn default() -> Self {
        Self {
            service_name: "myservice".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct LoggingConfig {
    /// The log level to use, this is a tracing env filter
    pub level: String,

    /// If we should use JSON logging
    pub json: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            json: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct AppConfig {
    /// Name of this instance
    pub name: String,

    /// The path to the config file.
    pub config_file: String,

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
            config_file: "config".to_string(),
            edge: EdgeConfig::default(),
            grpc: GrpcConfig::default(),
            logging: LoggingConfig::default(),
            redis: RedisConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn parse() -> Result<Self> {
        Ok(common::config::parse(&AppConfig::default().config_file)?)
    }
}
