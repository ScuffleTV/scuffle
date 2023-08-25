use std::net::SocketAddr;

use anyhow::Result;
use common::config::{LoggingConfig, NatsConfig, RedisConfig, TlsConfig};

#[derive(Debug, Clone, PartialEq, config::Config, serde::Deserialize)]
#[serde(default)]
/// The API is the backend for the Scuffle service
pub struct AppConfig {
    /// The path to the config file
    pub config_file: Option<String>,

    /// Name of this instance
    pub name: String,

    /// If we should export the GraphQL schema, if set to true, the schema will be exported to the stdout, and the program will exit.
    pub export_gql: bool,

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

    /// Redis configuration
    pub redis: RedisConfig,

    /// Nats configuration
    pub nats: NatsConfig,
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
pub struct TurnstileConfig {
    /// The Cloudflare Turnstile site key to use
    pub secret_key: String,

    /// The Cloudflare Turnstile url to use
    pub url: String,
}

impl Default for TurnstileConfig {
    fn default() -> Self {
        Self {
            secret_key: "1x0000000000000000000000000000000AA".to_string(),
            url: "https://challenges.cloudflare.com/turnstile/v0/siteverify".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, config::Config, serde::Deserialize)]
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

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            config_file: Some("config".to_string()),
            name: "scuffle-api".to_string(),
            export_gql: false,
            logging: LoggingConfig::default(),
            api: ApiConfig::default(),
            database: DatabaseConfig::default(),
            jwt: JwtConfig::default(),
            turnstile: TurnstileConfig::default(),
            redis: RedisConfig::default(),
            nats: NatsConfig::default(),
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
