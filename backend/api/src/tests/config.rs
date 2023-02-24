use crate::config::AppConfig;

#[test]
fn test_parse() {
    let config = AppConfig::parse().expect("Failed to parse config");
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

    let config = AppConfig::parse().expect("Failed to parse config");
    assert_eq!(config.log_level, "api=debug");
    assert_eq!(config.bind_address, "[::]:8081");
    assert_eq!(
        config.database_url,
        "postgres://postgres:postgres@localhost:5433/postgres"
    );
}

#[test]
fn test_parse_file() {
    let tmp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let config_file = tmp_dir.path().join("config.toml");

    std::fs::write(
        &config_file,
        r#"
log_level = "api=debug"
bind_address = "[::]:8081"
database_url = "postgres://postgres:postgres@localhost:5433/postgres"
"#,
    )
    .expect("Failed to write config file");

    std::env::set_var(
        "SCUF_CONFIG_FILE",
        config_file.to_str().expect("Failed to get str"),
    );

    let config = AppConfig::parse().expect("Failed to parse config");

    assert_eq!(config.log_level, "api=debug");
    assert_eq!(config.bind_address, "[::]:8081");
    assert_eq!(
        config.database_url,
        "postgres://postgres:postgres@localhost:5433/postgres"
    );
    assert_eq!(
        config.config_file,
        config_file.to_str().expect("Failed to get str")
    );
}

#[test]
fn test_parse_file_env() {
    let tmp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let config_file = tmp_dir.path().join("config.toml");

    std::fs::write(
        &config_file,
        r#"
log_level = "api=debug"
bind_address = "[::]:8081"
database_url = "postgres://postgres:postgres@localhost:5433/postgres"
"#,
    )
    .expect("Failed to write config file");

    std::env::set_var(
        "SCUF_CONFIG_FILE",
        config_file.to_str().expect("Failed to get str"),
    );
    std::env::set_var("SCUF_LOG_LEVEL", "api=info");

    let config = AppConfig::parse().expect("Failed to parse config");

    assert_eq!(config.log_level, "api=info");
    assert_eq!(config.bind_address, "[::]:8081");
    assert_eq!(
        config.database_url,
        "postgres://postgres:postgres@localhost:5433/postgres"
    );
    assert_eq!(
        config.config_file,
        config_file.to_str().expect("Failed to get str")
    );
}
