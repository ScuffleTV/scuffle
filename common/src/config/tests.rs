use super::*;

#[derive(Deserialize, Debug, Default)]
struct Config {
    foo: String,
    bar: String,
}

#[test]
fn test_parse() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let config_file = tmp_dir.path().join("config.toml");

    std::fs::write(
        &config_file,
        r#"
foo = "foo"
bar = "bar"
"#,
    )
    .unwrap();

    let config: Config = parse(config_file.to_str().unwrap()).unwrap();
    assert_eq!(config.foo, "foo");
    assert_eq!(config.bar, "bar");
}

#[test]
fn test_parse_env() {
    std::env::set_var("SCUF_FOO", "foo");
    std::env::set_var("SCUF_BAR", "bar");

    let config: Config = parse("").unwrap();
    assert_eq!(config.foo, "foo");
    assert_eq!(config.bar, "bar");
}
