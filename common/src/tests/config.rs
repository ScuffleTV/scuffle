use serde::Deserialize;

use crate::config::parse;

#[derive(Deserialize, Debug, Default)]
struct Config {
    foo: String,
    bar: String,
}

#[test]
fn test_parse() {
    let tmp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let config_file = tmp_dir.path().join("config.toml");

    std::fs::write(
        &config_file,
        r#"
foo = "foo"
bar = "bar"
"#,
    )
    .expect("Failed to write config file");

    let config: Config = parse(config_file.to_str().expect("failed to get config path"))
        .expect("Failed to parse config");
    assert_eq!(config.foo, "foo");
    assert_eq!(config.bar, "bar");
}

#[test]
fn test_parse_env() {
    std::env::set_var("SCUF_FOO", "foo");
    std::env::set_var("SCUF_BAR", "bar");

    let config: Config = parse("").expect("Failed to parse config");
    assert_eq!(config.foo, "foo");
    assert_eq!(config.bar, "bar");
}
