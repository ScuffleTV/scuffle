use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct AppConfig {
    /// The log level to use, this is a tracing env filter
    pub log_level: String,

    /// The path to the config file.
    pub config_file: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            log_level: "ingest=info".to_string(),
            config_file: "config".to_string(),
        }
    }
}

impl AppConfig {
    pub fn parse() -> Result<Self> {
        Ok(common::config::parse(&AppConfig::default().config_file)?)
    }
}
