use std::net::SocketAddr;

use common::config::TlsConfig;

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
