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
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            log_level: "api=info".to_string(),
            config_file: "config".to_string(),
            bind_address: "[::]:8080".to_string(),
            database_url: "postgres://postgres:postgres@localhost:5432/scuffle-dev".to_string(),
        }
    }
}

impl AppConfig {
    pub fn parse() -> Result<Self> {
        Ok(common::config::parse(&AppConfig::default().config_file)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let config = AppConfig::parse().unwrap();
        assert_eq!(config, AppConfig::default());
    }

    #[test]
    fn test_parse_env() {
        std::env::set_var("SCUF_LOG_LEVEL", "api=debug");
        std::env::set_var("SCUF_BIND_ADDRESS", "[::]:8081");
        std::env::set_var(
            "SCUF_DATABASE_URL",
            "postgres://postgres:postgres@localhost:5433/postgres",
        );

        let config = AppConfig::parse().unwrap();
        assert_eq!(config.log_level, "api=debug");
        assert_eq!(config.bind_address, "[::]:8081");
        assert_eq!(
            config.database_url,
            "postgres://postgres:postgres@localhost:5433/postgres"
        );
    }

    #[test]
    fn test_parse_file() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let config_file = tmp_dir.path().join("config.toml");

        std::fs::write(
            &config_file,
            r#"
log_level = "api=debug"
bind_address = "[::]:8081"
database_url = "postgres://postgres:postgres@localhost:5433/postgres"
"#,
        )
        .unwrap();

        std::env::set_var("SCUF_CONFIG_FILE", config_file.to_str().unwrap());

        let config = AppConfig::parse().unwrap();

        assert_eq!(config.log_level, "api=debug");
        assert_eq!(config.bind_address, "[::]:8081");
        assert_eq!(
            config.database_url,
            "postgres://postgres:postgres@localhost:5433/postgres"
        );
        assert_eq!(config.config_file, config_file.to_str().unwrap());
    }

    #[test]
    fn test_parse_file_env() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let config_file = tmp_dir.path().join("config.toml");

        std::fs::write(
            &config_file,
            r#"
log_level = "api=debug"
bind_address = "[::]:8081"
database_url = "postgres://postgres:postgres@localhost:5433/postgres"
"#,
        )
        .unwrap();

        std::env::set_var("SCUF_CONFIG_FILE", config_file.to_str().unwrap());
        std::env::set_var("SCUF_LOG_LEVEL", "api=info");

        let config = AppConfig::parse().unwrap();

        assert_eq!(config.log_level, "api=info");
        assert_eq!(config.bind_address, "[::]:8081");
        assert_eq!(
            config.database_url,
            "postgres://postgres:postgres@localhost:5433/postgres"
        );
        assert_eq!(config.config_file, config_file.to_str().unwrap());
    }
}
