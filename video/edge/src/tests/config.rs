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
    assert_eq!(
        config,
        AppConfig {
            config_file: None,
            ..Default::default()
        }
    );
}

#[serial]
#[test]
fn test_parse_env() {
    clear_env();

    std::env::set_var("SCUF_LOGGING_LEVEL", "edge=debug");
    std::env::set_var(
        "SCUF_DATABASE_URI",
        "postgres://postgres:postgres@localhost:5433/postgres",
    );

    let config = AppConfig::parse().expect("Failed to parse config");
    assert_eq!(config.logging.level, "edge=debug");
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
level = "edge=debug"

[edge]
bind_address = "0.0.0.0:8080"
"#,
    )
    .expect("Failed to write config file");

    std::env::set_var(
        "SCUF_CONFIG_FILE",
        config_file.to_str().expect("Failed to get str"),
    );

    let config = AppConfig::parse().expect("Failed to parse config");

    assert_eq!(config.logging.level, "edge=debug");
    assert_eq!(config.edge.bind_address, "0.0.0.0:8080".parse().unwrap());
    assert_eq!(
        config.config_file,
        Some(
            std::fs::canonicalize(config_file)
                .unwrap()
                .display()
                .to_string()
        )
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
level = "edge=debug"

[edge]
bind_address = "[::]:8080"
"#,
    )
    .expect("Failed to write config file");

    std::env::set_var(
        "SCUF_CONFIG_FILE",
        config_file.to_str().expect("Failed to get str"),
    );
    std::env::set_var("SCUF_LOGGING_LEVEL", "edge=info");

    let config = AppConfig::parse().expect("Failed to parse config");

    assert_eq!(config.logging.level, "edge=info");
    assert_eq!(config.edge.bind_address, "[::]:8080".parse().unwrap());
    assert_eq!(
        config.config_file,
        Some(
            std::fs::canonicalize(config_file)
                .unwrap()
                .display()
                .to_string()
        )
    );
}
