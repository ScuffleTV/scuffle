use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct AppConfig {
    /// The log level to use, this is a tracing env filter
    pub log_level: String,

    /// The path to the config file.
    pub config_file: String,

    /// Bind address for the API
    pub bind_address: String,

    /// The database URL to use
    pub database_url: String,

    /// The Redis URLs to use
    pub redis_urls: Vec<(String, u16)>,

    // The Redis username
    pub redis_username: String,

    // The Redis password
    pub redis_password: String,

    // Bool indicating wether to use Redis Sentiel or just Redis
    pub redis_sentinel: bool,

    /// The Cloudflare Turnstile site key to use
    pub turnstile_secret_key: String,

    /// The Cloudflare Turnstile url to use
    pub turnstile_url: String,

    /// JWT secret
    pub jwt_secret: String,

    /// JWT issuer
    pub jwt_issuer: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            config_file: "config".to_string(),
            bind_address: "[::]:8080".to_string(),
            database_url: "postgres://postgres:postgres@localhost:5432/scuffle-dev".to_string(),
            turnstile_secret_key: "DUMMY_KEY__SAMPLE_TEXT".to_string(),
            redis_urls: vec![("127.0.0.1".to_string(), 6379)],
            redis_username: "".to_string(),
            redis_password: "".to_string(),
            redis_sentinel: false,
            turnstile_url: "https://challenges.cloudflare.com/turnstile/v0/siteverify".to_string(),
            jwt_issuer: "scuffle".to_string(),
            jwt_secret: "scuffle".to_string(),
        }
    }
}

impl AppConfig {
    pub fn parse() -> Result<Self> {
        Ok(common::config::parse(&AppConfig::default().config_file)?)
    }
}
