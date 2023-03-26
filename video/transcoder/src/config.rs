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
            resolve_interval: 30,
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
            bind_address: "[::]:50053".to_string().parse().unwrap(),
            tls: None,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct IngestConfig {
    /// If we should use TLS for the API server
    pub tls: Option<TlsConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct RmqConfig {
    /// URI for RMQ
    pub uri: String,

    /// Stream name used for transcoder requests
    pub transcoder_queue: String,
}

impl Default for RmqConfig {
    fn default() -> Self {
        Self {
            uri: "amqp://rabbitmq:rabbitmq@localhost:5672/scuffle".to_string(),
            transcoder_queue: "transcoder".to_string(),
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
pub struct TranscoderConfig {
    /// The direcory to create unix sockets in
    pub socket_dir: String,
}

impl Default for TranscoderConfig {
    fn default() -> Self {
        Self {
            socket_dir: "/tmp".to_string(),
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
    pub api: ApiConfig,

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
            config_file: "config".to_string(),
            api: ApiConfig::default(),
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
        Ok(common::config::parse(&AppConfig::default().config_file)?)
    }
}
