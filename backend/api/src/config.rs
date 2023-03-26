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
pub struct RmqConfig {
    /// The URI to use for connecting to RabbitMQ
    pub uri: String,
}

impl Default for RmqConfig {
    fn default() -> Self {
        Self {
            uri: "amqp://rabbitmq:rabbitmq@localhost:5672/%2fscuffle".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct AppConfig {
    /// The path to the config file
    pub config_file: String,

    /// Name of this instance
    pub name: String,

    ///  The logging config
    pub logging: LoggingConfig,

    /// API Config
    pub api: ApiConfig,

    /// Database Config
    pub database: DatabaseConfig,

    /// Turnstile Config
    pub turnstile: TurnstileConfig,

    /// JWT Config
    pub jwt: JwtConfig,

    /// GRPC Config
    pub grpc: GrpcConfig,

    /// RMQ Config
    pub rmq: RmqConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct TurnstileConfig {
    /// The Cloudflare Turnstile site key to use
    pub secret_key: String,

    /// The Cloudflare Turnstile url to use
    pub url: String,
}

impl Default for TurnstileConfig {
    fn default() -> Self {
        Self {
            secret_key: "DUMMY_KEY__SAMPLE_TEXT".to_string(),
            url: "https://challenges.cloudflare.com/turnstile/v0/siteverify".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct JwtConfig {
    /// JWT secret
    pub secret: String,

    /// JWT issuer
    pub issuer: String,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            issuer: "scuffle".to_string(),
            secret: "scuffle".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
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
            config_file: "config".to_string(),
            name: "scuffle-api".to_string(),
            logging: LoggingConfig::default(),
            api: ApiConfig::default(),
            database: DatabaseConfig::default(),
            grpc: GrpcConfig::default(),
            jwt: JwtConfig::default(),
            turnstile: TurnstileConfig::default(),
            rmq: RmqConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn parse() -> Result<Self> {
        Ok(common::config::parse(&AppConfig::default().config_file)?)
    }
}
