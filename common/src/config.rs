use std::sync::Arc;

use crate::logging;

#[derive(Debug, Clone, Default, PartialEq, config::Config, serde::Deserialize)]
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

#[derive(Debug, Clone, PartialEq, config::Config, serde::Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    /// The log level to use, this is a tracing env filter
    pub level: String,

    /// What logging mode we should use
    pub mode: logging::Mode,
}

impl ::config::Config for logging::Mode {
    fn graph() -> Arc<::config::KeyGraph> {
        Arc::new(::config::KeyGraph::String)
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            mode: logging::Mode::Default,
        }
    }
}

#[derive(Debug, Clone, PartialEq, config::Config, serde::Deserialize)]
#[serde(default)]
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

#[derive(Debug, Clone, PartialEq, config::Config, serde::Deserialize)]
#[serde(default)]
pub struct RedisSentinelConfig {
    /// The master group name
    pub service_name: String,
}

#[derive(Debug, Clone, PartialEq, config::Config, serde::Deserialize)]
#[serde(default)]
pub struct NatsConfig {
    /// The URI to use for connecting to Nats
    pub servers: Vec<String>,

    /// The username to use for authentication (user-pass auth)
    pub username: Option<String>,

    /// The password to use for authentication (user-pass auth)
    pub password: Option<String>,

    /// The token to use for authentication (token auth)
    pub token: Option<String>,

    /// The TLS configuration (can be used for mTLS)
    pub tls: Option<TlsConfig>,
}

impl Default for NatsConfig {
    fn default() -> Self {
        Self {
            servers: vec!["localhost:4222".into()],
            token: None,
            password: None,
            tls: None,
            username: None,
        }
    }
}

impl Default for RedisSentinelConfig {
    fn default() -> Self {
        Self {
            service_name: "myservice".to_string(),
        }
    }
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

pub fn parse<C: config::Config + 'static>(
    enable_cli: bool,
    config_file: Option<String>,
) -> config::Result<(C, Option<String>)> {
    let mut builder = config::ConfigBuilder::new();

    if enable_cli {
        builder.add_source_with_priority(config::sources::CliSource::new()?, 3);
    }

    builder.add_source_with_priority(config::sources::EnvSource::with_prefix("SCUF")?, 2);

    let key = builder.parse_key::<Option<String>>("config_file")?;

    let key_provided = key.is_some();

    let mut config_path = None;

    if let Some(path) = key.or(config_file) {
        match config::sources::FileSource::with_path(path) {
            Ok(source) => {
                config_path = Some(source.location().to_string());
                builder.add_source_with_priority(source, 1);
            }
            Err(err) => {
                if key_provided || !err.is_io() {
                    return Err(err);
                }

                tracing::debug!("failed to load config file: {}", err);
            }
        }
    }

    Ok((
        builder.build()?,
        config_path.map(|p| std::fs::canonicalize(p).unwrap().display().to_string()),
    ))
}
