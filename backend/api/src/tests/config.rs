use serial_test::serial;

use crate::config::AppConfig;

fn clear_env() {
    for (key, _) in std::env::vars() {
        if key.starts_with("SCUF_") {
            std::env::remove_var(key);
        }
    }
}

#[serial]
#[test]
fn test_parse() {
    clear_env();

    let config = AppConfig::parse().expect("Failed to parse config");
    assert_eq!(config, AppConfig::default());
}

#[serial]
#[test]
fn test_parse_env() {
    clear_env();

    std::env::set_var("SCUF_LOGGING__LEVEL", "api=debug");
    std::env::set_var("SCUF_API__BIND_ADDRESS", "[::]:8081");
    std::env::set_var(
        "SCUF_DATABASE__URI",
        "postgres://postgres:postgres@localhost:5433/postgres",
    );

    let config = AppConfig::parse().expect("Failed to parse config");
    assert_eq!(config.logging.level, "api=debug");
    assert_eq!(config.api.bind_address, "[::]:8081".parse().unwrap());
    assert_eq!(
        config.database.uri,
        "postgres://postgres:postgres@localhost:5433/postgres"
    );
}

#[serial]
#[test]
fn test_parse_file() {
    clear_env();

    let tmp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let config_file = tmp_dir.path().join("config.toml");

    std::fs::write(
        &config_file,
        r#"
[logging]
level = "api=debug"

[api]
bind_address = "[::]:8081"

[database]
uri = "postgres://postgres:postgres@localhost:5433/postgres"
"#,
    )
    .expect("Failed to write config file");

    std::env::set_var(
        "SCUF_CONFIG_FILE",
        config_file.to_str().expect("Failed to get str"),
    );

    let config = AppConfig::parse().expect("Failed to parse config");

    assert_eq!(config.logging.level, "api=debug");
    assert_eq!(config.api.bind_address, "[::]:8081".parse().unwrap());
    assert_eq!(
        config.database.uri,
        "postgres://postgres:postgres@localhost:5433/postgres"
    );
    assert_eq!(
        config.config_file,
        config_file.to_str().expect("Failed to get str")
    );
}

#[serial]
#[test]
fn test_parse_file_env() {
    clear_env();

    let tmp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let config_file = tmp_dir.path().join("config.toml");

    std::fs::write(
        &config_file,
        r#"
[logging]
level = "api=debug"

[api]
bind_address = "[::]:8081"

[database]
uri = "postgres://postgres:postgres@localhost:5433/postgres"
"#,
    )
    .expect("Failed to write config file");

    std::env::set_var(
        "SCUF_CONFIG_FILE",
        config_file.to_str().expect("Failed to get str"),
    );
    std::env::set_var("SCUF_LOGGING__LEVEL", "api=info");

    let config = AppConfig::parse().expect("Failed to parse config");

    assert_eq!(config.logging.level, "api=info");
    assert_eq!(config.api.bind_address, "[::]:8081".parse().unwrap());
    assert_eq!(
        config.database.uri,
        "postgres://postgres:postgres@localhost:5433/postgres"
    );
    assert_eq!(
        config.config_file,
        config_file.to_str().expect("Failed to get str")
    );
}
