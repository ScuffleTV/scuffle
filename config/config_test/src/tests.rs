//! The derive macro can't be tested in this file because it cannot be part of the config crate.
//! This has to do with the way the macro generates code. It refers to items in the config crate with `::config`
//! which is not available in the config crate itself.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    sync::Arc,
};

use config::{sources, Config, ConfigBuilder, Key, KeyGraph, KeyPath, Source, Value};

fn clear_env() {
    for (key, _) in std::env::vars() {
        if key.starts_with("SCUF_") {
            std::env::remove_var(key);
        }
    }
}

#[derive(Debug, PartialEq, serde::Deserialize)]
struct DummyConfig {
    enabled: bool,
    logging: LoggingConfig,
}

// Can be generated with Config derive macro
impl Config for DummyConfig {
    fn graph() -> Arc<KeyGraph> {
        let builder = KeyGraph::builder::<Self>();
        if let Some(graph) = builder.get() {
            return graph;
        }

        let mut keys = BTreeMap::new();

        keys.insert("enabled".to_string(), Key::new(bool::graph()));
        keys.insert("logging".to_string(), Key::new(LoggingConfig::graph()));

        builder.build(KeyGraph::Struct(keys))
    }
}

#[derive(Debug, PartialEq, serde::Deserialize)]
struct LoggingConfig {
    level: String,
    json: bool,
}

impl Config for LoggingConfig {
    fn graph() -> Arc<KeyGraph> {
        let builder = KeyGraph::builder::<Self>();
        if let Some(graph) = builder.get() {
            return graph;
        }

        let mut keys = BTreeMap::new();

        keys.insert("level".to_string(), Key::new(String::graph()));
        keys.insert("json".to_string(), Key::new(bool::graph()));

        builder.build(KeyGraph::Struct(keys))
    }
}

#[test]
fn env() {
    // With custom prefix and default joiner
    clear_env();
    std::env::set_var("SCUF_ENABLED", "true");
    let config = sources::EnvSource::<DummyConfig>::with_prefix("SCUF").unwrap();
    assert_eq!(
        config.get_key(&"enabled".into()).unwrap().unwrap(),
        Value::Bool(true),
    );

    // With no prefix and custom joiner
    clear_env();
    std::env::set_var("LOGGING__JSON", "false");
    let config = sources::EnvSource::<DummyConfig>::with_joiner(None, "__").unwrap();
    assert_eq!(
        config.get_key(&"logging.json".into()).unwrap().unwrap(),
        Value::Bool(false),
    );

    // With custom prefix and custom joiner
    clear_env();
    std::env::set_var("LOGGING_JSON", "true");
    let config = sources::EnvSource::<DummyConfig>::new().unwrap();
    assert_eq!(
        config.get_key(&"logging.json".into()).unwrap().unwrap(),
        Value::Bool(true),
    );
}

#[test]
fn file() {
    let data: &[u8] = br#"
    [logging]
    level = "test_value"
    "#;
    let config = sources::FileSource::<DummyConfig>::toml(data).unwrap();
    assert_eq!(
        config.get_key(&"logging.level".into()).unwrap().unwrap(),
        Value::String("test_value".to_string())
    );
    assert_eq!(config.get_key(&"test.not_defined".into()).unwrap(), None);
}

#[test]
fn cli() {
    let matches = sources::cli::generate_command::<DummyConfig>()
        .unwrap()
        .get_matches_from(vec![
            "cli_test",
            "--enabled",
            "true",
            "--logging.level",
            "INFO",
            "--logging.json",
            "false",
        ]);
    let cli = sources::CliSource::<DummyConfig>::with_matches(matches).unwrap();
    assert_eq!(
        cli.get_key(&"enabled".into()).unwrap().unwrap(),
        Value::Bool(true),
    );
    assert_eq!(
        cli.get_key(&"logging.level".into()).unwrap().unwrap(),
        Value::String("INFO".to_string()),
    );
    assert_eq!(
        cli.get_key(&"logging.json".into()).unwrap().unwrap(),
        Value::Bool(false),
    );
}

#[test]
fn config_builder() {
    let mut builder: ConfigBuilder<DummyConfig> = ConfigBuilder::new();
    let data: &[u8] = br#"
    enabled = true
    [logging]
    level = "INFO"
    json = false
    "#;
    builder.add_source(sources::FileSource::toml(data).unwrap());

    assert_eq!(
        builder.build().unwrap(),
        DummyConfig {
            enabled: true,
            logging: LoggingConfig {
                level: "INFO".to_string(),
                json: false
            }
        }
    );
}

#[test]
fn config_derive_cli() {
    #[derive(Debug, Default, PartialEq, serde::Deserialize, config::Config)]
    #[serde(default)]
    struct AppConfig {
        value: String,
        enabled: bool,
    }

    let mut command = sources::cli::generate_command::<AppConfig>().unwrap();
    let help = command.render_long_help();
    assert_eq!(help.to_string(), "Usage: config_test [OPTIONS]\n\nOptions:\n      --enabled [<enabled>]\n          [possible values: true, false]\n\n      --value <value>\n          \n\n  -h, --help\n          Print help\n\n  -V, --version\n          Print version\n");
    let matches =
        command.get_matches_from(vec!["config_test", "--enabled", "true", "--value", "test"]);
    let cli = sources::CliSource::<AppConfig>::with_matches(matches).unwrap();

    let value = cli.get_key(&"enabled".into()).unwrap().unwrap();
    assert_eq!(value, Value::Bool(true));

    let value = cli.get_key(&"value".into()).unwrap().unwrap();
    assert_eq!(value, Value::String("test".to_string()));
}

#[test]
fn config_derive_nested_cli() {
    #[derive(Debug, Default, PartialEq, serde::Deserialize, config::Config)]
    #[serde(default)]
    struct AppConfig {
        value: String,
        enabled: bool,
        sub: SubConfig,
    }

    #[derive(Debug, PartialEq, serde::Deserialize, config::Config)]
    #[serde(default)]
    struct SubConfig {
        duration: std::time::Duration,
        time: std::time::SystemTime,
    }

    impl Default for SubConfig {
        fn default() -> Self {
            Self {
                duration: std::time::Duration::from_secs(0),
                time: std::time::SystemTime::UNIX_EPOCH,
            }
        }
    }

    let mut command = sources::cli::generate_command::<AppConfig>().unwrap();
    let help = command.render_long_help();
    assert_eq!(help.to_string(), "Usage: config_test [OPTIONS]\n\nOptions:\n      --enabled [<enabled>]\n          [possible values: true, false]\n\n      --sub.duration <sub.duration>\n          \n\n      --sub.time <sub.time>\n          \n\n      --value <value>\n          \n\n  -h, --help\n          Print help\n\n  -V, --version\n          Print version\n");
    let matches = command.get_matches_from(vec![
        "config_test",
        "--enabled",
        "true",
        "--value",
        "test",
        "--sub.duration",
        "1d32s",
        "--sub.time",
        "2018-01-01T12:53:00Z",
    ]);
    let cli = sources::CliSource::<AppConfig>::with_matches(matches).unwrap();

    let value = cli.get_key(&"enabled".into()).unwrap().unwrap();
    assert_eq!(value, Value::Bool(true));

    let value = cli.get_key(&"value".into()).unwrap().unwrap();
    assert_eq!(value, Value::String("test".to_string()));

    // This happens to be what serde does when deserializing a Duration
    // However we allow the user to specify a human readable duration '10s' rather than the map representation
    let value = cli.get_key(&"sub.duration".into()).unwrap().unwrap();
    assert_eq!(
        value,
        Value::Map(
            vec![
                (Value::String("secs".to_string()), Value::U64(86432)),
                (Value::String("nanos".to_string()), Value::U32(0))
            ]
            .into_iter()
            .collect()
        )
    );
    let duration = config::parse_key::<std::time::Duration>(value).unwrap();
    assert_eq!(
        duration,
        std::time::Duration::from_secs(32) + std::time::Duration::from_secs(24 * 60 * 60)
    );

    let value = cli.get_key(&"sub.time".into()).unwrap().unwrap();
    assert_eq!(
        value,
        Value::Map(
            vec![
                (
                    Value::String("nanos_since_epoch".to_string()),
                    Value::U32(0)
                ),
                (
                    Value::String("secs_since_epoch".to_string()),
                    Value::U64(1514811180)
                )
            ]
            .into_iter()
            .collect()
        )
    );

    let time = config::parse_key::<std::time::SystemTime>(value).unwrap();
    assert_eq!(
        time,
        std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1514811180)
    );
}

#[test]
fn config_derive_cyclic() {
    #[derive(Debug, Default, PartialEq, serde::Deserialize, config::Config)]
    #[serde(default)]
    struct AppConfig {
        sub: SubConfig,
        value: Option<String>,
    }

    #[derive(Debug, Default, PartialEq, serde::Deserialize, config::Config)]
    #[serde(default)]
    struct SubConfig {
        #[config(cli(skip))]
        app: Option<Box<AppConfig>>,
        value2: Arc<u32>,
    }

    let mut command = sources::cli::generate_command::<AppConfig>().unwrap();
    let help = command.render_long_help();

    assert_eq!(help.to_string(), "Usage: config_test [OPTIONS]\n\nOptions:\n      --sub.value-2 <sub.value2>\n          \n\n      --value [<value>]\n          \n\n  -h, --help\n          Print help\n\n  -V, --version\n          Print version\n");

    let matches = command.get_matches_from(vec!["config_test", "--sub.value-2", "10"]);
    let cli = sources::CliSource::<AppConfig>::with_matches(matches).unwrap();

    let value = cli.get_key(&"sub.value2".into()).unwrap().unwrap();
    assert_eq!(value, Value::U32(10));

    let value = cli.get_key(&"value".into()).unwrap();
    assert_eq!(value, None);

    let value = cli.get_key(&"sub.app".into()).unwrap();
    assert_eq!(value, None);

    // We now can try deserializing the config from json
    let data: &[u8] = br#"
    {
        "sub": {
            "value2": 32,
            "app": {
                "value": "test",
                "sub": {
                    "value2": 10
                }
            }
        }
    }"#;

    let config = sources::FileSource::<AppConfig>::json(data).unwrap();
    assert_eq!(
        config.get_key(&"sub.value2".into()).unwrap().unwrap(),
        Value::U32(32),
    );

    let value = config.get_key(&"value".into()).unwrap();
    assert_eq!(value, None);

    let value = config.get_key(&"sub.app".into()).unwrap().unwrap();
    assert_eq!(
        value,
        Value::Option(Some(Box::new(Value::Map(
            vec![
                (
                    Value::String("sub".to_string()),
                    Value::Map(
                        vec![(Value::String("value2".to_string()), Value::U32(10))]
                            .into_iter()
                            .collect()
                    )
                ),
                (
                    Value::String("value".to_string()),
                    Value::Option(Some(Box::new(Value::String("test".to_string()))))
                )
            ]
            .into_iter()
            .collect()
        ))))
    );

    let value = config.get_key(&KeyPath::root()).unwrap().unwrap();
    let config: AppConfig = config::parse_key(value).unwrap();

    assert_eq!(
        config,
        AppConfig {
            sub: SubConfig {
                value2: Arc::new(32),
                app: Some(Box::new(AppConfig {
                    value: Some("test".to_string()),
                    sub: SubConfig {
                        value2: Arc::new(10),
                        app: None,
                    }
                }))
            },
            value: None,
        }
    );
}

#[test]
fn config_derive_env_builder() {
    // Clear env before running tests
    for (key, _) in std::env::vars() {
        std::env::remove_var(key);
    }

    // Set some env variables to see if priority works
    std::env::set_var("TEST_VALUE", "env");
    std::env::set_var("TEST_ENABLED", "off");
    std::env::set_var("TEST_SUB_VALUE", "env");
    std::env::set_var("TEST_SUB_COUNT", "1,2,3");

    #[derive(Debug, Default, PartialEq, serde::Deserialize, config::Config)]
    #[serde(default)]
    struct AppConfig {
        value: String,
        enabled: bool,
        name: Option<String>,
        sub: SubConfig,
    }

    #[derive(Debug, Default, PartialEq, serde::Deserialize, config::Config)]
    #[serde(default)]
    struct SubConfig {
        value: String,
        count: Vec<u32>,
    }

    let mut builder: ConfigBuilder<AppConfig> = ConfigBuilder::new();
    builder.add_source(sources::EnvSource::with_prefix("TEST").unwrap());
    builder.add_source(
        sources::FileSource::toml(
            br#"
    value = "test"
    enabled = true
    name = "test"
    "#
            .as_slice(),
        )
        .unwrap(),
    );

    let config = builder.build().unwrap();
    assert_eq!(
        config,
        AppConfig {
            value: "env".to_string(),
            enabled: false,
            name: Some("test".to_string()),
            sub: SubConfig {
                value: "env".to_string(),
                count: vec![1, 2, 3],
            }
        }
    );
}

#[test]
fn cli_vector_derive() {
    #[derive(Debug, Default, PartialEq, serde::Deserialize, config::Config)]
    #[serde(default)]
    struct AppConfig {
        value: String,
        enabled: bool,
        count: Vec<u32>,
    }

    let mut command = sources::cli::generate_command::<AppConfig>().unwrap();
    let help = command.render_long_help();
    assert_eq!(help.to_string(), "Usage: config_test [OPTIONS]\n\nOptions:\n      --count <count>...\n          \n\n      --enabled [<enabled>]\n          [possible values: true, false]\n\n      --value <value>\n          \n\n  -h, --help\n          Print help\n\n  -V, --version\n          Print version\n");
    let matches = command.get_matches_from(vec![
        "config_test",
        "--enabled",
        "true",
        "--value",
        "test",
        "--count",
        "1",
        "2",
        "3",
    ]);
    let cli = sources::CliSource::<AppConfig>::with_matches(matches).unwrap();

    let value = cli.get_key(&"enabled".into()).unwrap().unwrap();
    assert_eq!(value, Value::Bool(true));

    let value = cli.get_key(&"value".into()).unwrap().unwrap();
    assert_eq!(value, Value::String("test".to_string()));

    let value = cli.get_key(&"count".into()).unwrap().unwrap();
    assert_eq!(
        value,
        Value::Seq(vec![Value::U32(1), Value::U32(2), Value::U32(3)])
    );
    let count = config::parse_key::<Vec<u32>>(value).unwrap();
    assert_eq!(count, vec![1, 2, 3]);
}

#[test]
fn fringe_types() {
    // The reason we explicityly set the graph type is because
    // we want to test the validate from graph function which
    // is only called when we dont already know the validator type
    // Normally when you use the derive macro the graph type is known
    // by the config::Config trait since it has a validator function.

    // Setting these graph types explicitly does not change the behaviour
    // just adds an extra indirection by passing to validate_from_graph before calling
    // the validate on the type itself.

    #[derive(Debug, Default, PartialEq, serde::Deserialize, config::Config)]
    #[serde(default)]
    struct AppConfig {
        #[config(graph = "String::graph()")]
        value: String,
        #[config(graph = "bool::graph()")]
        enabled: bool,
        #[config(graph = "Vec::<u32>::graph()")]
        count: Vec<u32>,
        #[config(graph = "HashMap::<String, u32>::graph()")]
        map: HashMap<String, u32>,
        #[config(graph = "HashSet::<u32>::graph()")]
        set: HashSet<u32>,
        #[config(graph = "f32::graph()")]
        float: f32,
        #[config(graph = "f64::graph()")]
        double: f64,
        #[config(graph = "i32::graph()")]
        int: i32,
        #[config(graph = "u32::graph()")]
        uint: u32,
        #[config(graph = "i8::graph()")]
        int8: i8,
        #[config(graph = "u8::graph()")]
        uint8: u8,
        #[config(graph = "i16::graph()")]
        int16: i16,
        #[config(graph = "u16::graph()")]
        uint16: u16,
        #[config(graph = "i64::graph()")]
        int64: i64,
        #[config(graph = "u64::graph()")]
        uint64: u64,
        #[config(graph = "isize::graph()")]
        isize: isize,
        #[config(graph = "usize::graph()")]
        usize: usize,
        #[config(graph = "char::graph()")]
        char: char,
        #[config(graph = "<()>::graph()")]
        unit: (),
        #[config(graph = "Vec::<()>::graph()")]
        seq: Vec<()>,
        #[config(graph = "Option::<()>::graph()")]
        option: Option<()>,
        #[config(graph = "Option::<Option<()>>::graph()")]
        option_option: Option<Option<()>>,
        array: [u32; 3],
        box_str: Box<str>,

        nested_array: [[u32; 3]; 3],
        nested_seq: Vec<Vec<()>>,
        nested_map: HashMap<String, HashMap<String, ()>>,

        #[config(graph = "BTreeSet::<u32>::graph()")]
        btreeset: BTreeSet<u32>,
        #[config(graph = "BTreeMap::<String, u32>::graph()")]
        btreemap: BTreeMap<String, u32>,

        // Note we do not want to pass a custom graph type for these
        // because in this case the validator actually transforms the type provided
        // We wrap them in Option because the Default trait is not implemented for them
        // and I am too lazy to implement it.
        ip_addr: Option<std::net::IpAddr>,
        ip_v4_addr: Option<std::net::Ipv4Addr>,
        ip_v6_addr: Option<std::net::Ipv6Addr>,
        socket_addr: Option<std::net::SocketAddr>,
        socket_v4_addr: Option<std::net::SocketAddrV4>,
        socket_v6_addr: Option<std::net::SocketAddrV6>,
        duration: Option<std::time::Duration>,
        time: Option<std::time::SystemTime>,

        phantom: std::marker::PhantomData<u32>,

        #[config(graph = "AppConfig::graph()")]
        reference: Option<Box<AppConfig>>,

        reference2: Box<Option<AppConfig>>,

        non_string_key: HashMap<u32, u32>,
    }

    let data = br#"
    value: test
    enabled: true
    count: [1, 2, 3]
    map: { test: 1 }
    set: [1, 2, 3]
    float: 1.0
    double: 2.0
    int: 1
    uint: 2
    int8: 3
    uint8: 4
    int16: 5
    uint16: 6
    int64: 7
    uint64: 8
    isize: 9
    usize: 10
    char: a
    unit: null
    seq: [null]
    option: null
    option_option: null
    array: [1, 2, 3]
    box_str: test

    nested_array: [[1, 2, 3], [4, 5, 6], [7, 8, 9]]
    nested_seq: [[null], [null], [null]]
    nested_map: { test: { test: null } }

    btreeset: [1, 2, 3]
    btreemap: { test: 152 }

    ip_addr: 0.0.0.0
    ip_v4_addr: 192.168.2.11
    ip_v6_addr: 2001:db8::1
    socket_addr: 0.0.0.0:0
    socket_v4_addr: 192.168.2.122:1234
    socket_v6_addr: "[2001:db8::1]:1234"
    duration: 1d32s
    time: 2018-01-01T12:53:00Z

    phantom: 0

    reference: { value: test }
    reference2: { value: test2 }

    non_string_key: { 1: 2 }

    "#
    .as_slice();

    let config = sources::FileSource::<AppConfig>::yaml(data).unwrap();

    let value = config.get_key(&KeyPath::root()).unwrap().unwrap();
    let config: AppConfig = config::parse_key(value).unwrap();

    assert_eq!(
        config,
        AppConfig {
            value: "test".to_string(),
            enabled: true,
            count: vec![1, 2, 3],
            map: vec![("test".to_string(), 1)].into_iter().collect(),
            set: vec![1, 2, 3].into_iter().collect(),
            float: 1.0,
            double: 2.0,
            int: 1,
            uint: 2,
            int8: 3,
            uint8: 4,
            int16: 5,
            uint16: 6,
            int64: 7,
            uint64: 8,
            isize: 9,
            usize: 10,
            char: 'a',
            unit: (),
            seq: vec![()],
            option: None,
            option_option: None,
            array: [1, 2, 3],
            box_str: "test".to_string().into_boxed_str(),
            nested_array: [[1, 2, 3], [4, 5, 6], [7, 8, 9]],
            nested_seq: vec![vec![()], vec![()], vec![()]],
            nested_map: vec![(
                "test".to_string(),
                vec![("test".to_string(), ())].into_iter().collect()
            )]
            .into_iter()
            .collect(),
            btreemap: vec![("test".to_string(), 152)].into_iter().collect(),
            btreeset: vec![1, 2, 3].into_iter().collect(),

            ip_addr: Some(std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0))),
            ip_v4_addr: Some(std::net::Ipv4Addr::new(192, 168, 2, 11)),
            ip_v6_addr: Some(std::net::Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)),
            socket_addr: Some(std::net::SocketAddr::new(
                std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
                0
            )),
            socket_v4_addr: Some(std::net::SocketAddrV4::new(
                std::net::Ipv4Addr::new(192, 168, 2, 122),
                1234
            )),
            socket_v6_addr: Some(std::net::SocketAddrV6::new(
                std::net::Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1),
                1234,
                0,
                0
            )),
            duration: Some(
                std::time::Duration::from_secs(32) + std::time::Duration::from_secs(24 * 60 * 60)
            ),
            time: Some(
                std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1514811180)
            ),
            phantom: std::marker::PhantomData,
            reference: Some(Box::new(AppConfig {
                value: "test".to_string(),
                ..Default::default()
            })),
            reference2: Box::new(Some(AppConfig {
                value: "test2".to_string(),
                ..Default::default()
            })),

            non_string_key: vec![(1, 2)].into_iter().collect(),
        }
    )
}

#[test]
fn config_derive_nested() {
    #[derive(Debug, Default, PartialEq, serde::Deserialize, config::Config)]
    #[serde(default)]
    struct AppConfig {
        value: String,
        enabled: bool,
        sub: SubConfig,
    }

    #[derive(Debug, Default, PartialEq, serde::Deserialize, config::Config)]
    #[serde(default)]
    struct SubConfig {
        value: String,
        count: Vec<u32>,
    }

    let mut builder: ConfigBuilder<AppConfig> = ConfigBuilder::new();
    builder.add_source(
        sources::FileSource::toml(
            br#"
    value = "test"
    enabled = true
    [sub]
    value = "test"
    count = [1, 2, 3]
    "#
            .as_slice(),
        )
        .unwrap(),
    );

    let config = builder.build().unwrap();
    assert_eq!(
        config,
        AppConfig {
            value: "test".to_string(),
            enabled: true,
            sub: SubConfig {
                value: "test".to_string(),
                count: vec![1, 2, 3],
            }
        }
    );
}

#[test]
fn cli_fringe_types() {
    #[derive(Debug, Default, PartialEq, serde::Deserialize, config::Config)]
    #[serde(default)]
    struct AppConfig {
        string: String,
        string_seq: Vec<String>,
        bool: bool,
        bool_seq: Vec<bool>,
        float: f32,
        float_seq: Vec<f32>,
        double: f64,
        double_seq: Vec<f64>,
        int: i32,
        int_seq: Vec<i32>,
        uint: u32,
        uint_seq: Vec<u32>,
        int8: i8,
        int8_seq: Vec<i8>,
        uint8: u8,
        uint8_seq: Vec<u8>,
        int16: i16,
        int16_seq: Vec<i16>,
        uint16: u16,
        uint16_seq: Vec<u16>,
        int64: i64,
        int64_seq: Vec<i64>,
        uint64: u64,
        uint64_seq: Vec<u64>,
        isize: isize,
        isize_seq: Vec<isize>,
        usize: usize,
        usize_seq: Vec<usize>,
        char: char,
        char_seq: Vec<char>,
        unit: (),
        unit_seq: Vec<()>,
    }

    let mut command = sources::cli::generate_command::<AppConfig>().unwrap();
    let help = command.render_long_help();

    assert_eq!(help.to_string(), "Usage: config_test [OPTIONS]\n\nOptions:\n      --bool [<bool>]\n          [possible values: true, false]\n\n      --bool-seq [<bool_seq>...]\n          [possible values: true, false]\n\n      --char <char>\n          \n\n      --char-seq <char_seq>...\n          \n\n      --double <double>\n          \n\n      --double-seq <double_seq>...\n          \n\n      --float <float>\n          \n\n      --float-seq <float_seq>...\n          \n\n      --int <int>\n          \n\n      --int-16 <int16>\n          \n\n      --int-16-seq <int16_seq>...\n          \n\n      --int-64 <int64>\n          \n\n      --int-64-seq <int64_seq>...\n          \n\n      --int-8 <int8>\n          \n\n      --int-8-seq <int8_seq>...\n          \n\n      --int-seq <int_seq>...\n          \n\n      --isize <isize>\n          \n\n      --isize-seq <isize_seq>...\n          \n\n      --string <string>\n          \n\n      --string-seq <string_seq>...\n          \n\n      --uint <uint>\n          \n\n      --uint-16 <uint16>\n          \n\n      --uint-16-seq <uint16_seq>...\n          \n\n      --uint-64 <uint64>\n          \n\n      --uint-64-seq <uint64_seq>...\n          \n\n      --uint-8 <uint8>\n          \n\n      --uint-8-seq <uint8_seq>...\n          \n\n      --uint-seq <uint_seq>...\n          \n\n      --unit\n          \n\n      --unit-seq [<unit_seq>...]\n          [possible values: true, false]\n\n      --usize <usize>\n          \n\n      --usize-seq <usize_seq>...\n          \n\n  -h, --help\n          Print help\n\n  -V, --version\n          Print version\n");

    let matches = command.get_matches_from(vec![
        "config_test",
        "--string",
        "test",
        "--bool",
        "true",
        "--float",
        "1.0",
        "--double",
        "2.0",
        "--int",
        "1",
        "--uint",
        "2",
        "--int-8",
        "3",
        "--uint-8",
        "4",
        "--int-16",
        "5",
        "--uint-16",
        "6",
        "--int-64",
        "7",
        "--uint-64",
        "8",
        "--isize",
        "9",
        "--usize",
        "10",
        "--char",
        "a",
        "--unit",
        "--string-seq",
        "test",
        "--bool-seq",
        "true",
        "--float-seq",
        "1.0",
        "--double-seq",
        "2.0",
        "--int-seq",
        "1",
        "--uint-seq",
        "2",
        "--int-8-seq",
        "3",
        "--uint-8-seq",
        "4",
        "--int-16-seq",
        "5",
        "--uint-16-seq",
        "6",
        "--int-64-seq",
        "7",
        "--uint-64-seq",
        "8",
        "--isize-seq",
        "9",
        "--usize-seq",
        "10",
        "--char-seq",
        "a",
        "--unit-seq",
        "true",
        "true",
    ]);

    let cli = sources::CliSource::<AppConfig>::with_matches(matches).unwrap();

    let mut builder = ConfigBuilder::new();
    builder.add_source(cli);

    let config: AppConfig = builder.build().unwrap();

    assert_eq!(
        config,
        AppConfig {
            string: "test".to_string(),
            bool: true,
            float: 1.0,
            double: 2.0,
            int: 1,
            uint: 2,
            int8: 3,
            uint8: 4,
            int16: 5,
            uint16: 6,
            int64: 7,
            uint64: 8,
            isize: 9,
            usize: 10,
            char: 'a',
            unit: (),
            string_seq: vec!["test".to_string()],
            bool_seq: vec![true],
            float_seq: vec![1.0],
            double_seq: vec![2.0],
            int_seq: vec![1],
            uint_seq: vec![2],
            int8_seq: vec![3],
            uint8_seq: vec![4],
            int16_seq: vec![5],
            uint16_seq: vec![6],
            int64_seq: vec![7],
            uint64_seq: vec![8],
            isize_seq: vec![9],
            usize_seq: vec![10],
            char_seq: vec!['a'],
            unit_seq: vec![(), ()],
        }
    );
}

#[test]
fn cli_comments() {
    #[derive(Debug, Default, PartialEq, serde::Deserialize, config::Config)]
    #[serde(default)]
    struct AppConfig {
        /// This is a comment about value
        value: String,
        /// This is a comment about enabled
        enabled: bool,
        /// This is a comment about sub
        sub: SubConfig,
    }

    #[derive(Debug, Default, PartialEq, serde::Deserialize, config::Config)]
    #[serde(default)]
    struct SubConfig {
        #[doc = "This is a comment about sub.value"]
        value: String,
        #[doc = "This is a comment about sub.count"]
        count: Vec<u32>,
    }

    let mut command = sources::cli::generate_command::<AppConfig>().unwrap();
    let help = command.render_long_help();
    assert_eq!(help.to_string(), "Usage: config_test [OPTIONS]\n\nOptions:\n      --enabled [<enabled>]\n           This is a comment about enabled\n          \n          [possible values: true, false]\n\n      --sub.count <sub.count>...\n          This is a comment about sub.count\n\n      --sub.value <sub.value>\n          This is a comment about sub.value\n\n      --value <value>\n           This is a comment about value\n\n  -h, --help\n          Print help\n\n  -V, --version\n          Print version\n");
    let matches = command.get_matches_from(vec![
        "config_test",
        "--enabled",
        "true",
        "--value",
        "test",
        "--sub.count",
        "1",
        "2",
        "3",
    ]);
    let cli = sources::CliSource::<AppConfig>::with_matches(matches).unwrap();

    let value = cli.get_key(&"enabled".into()).unwrap().unwrap();
    assert_eq!(value, Value::Bool(true));

    let value = cli.get_key(&"value".into()).unwrap().unwrap();
    assert_eq!(value, Value::String("test".to_string()));

    let value = cli.get_key(&"sub.count".into()).unwrap().unwrap();
    assert_eq!(
        value,
        Value::Seq(vec![Value::U32(1), Value::U32(2), Value::U32(3)])
    );
    let count = config::parse_key::<Vec<u32>>(value).unwrap();
    assert_eq!(count, vec![1, 2, 3]);
}

#[test]
fn get_sequence_key() {
    #[derive(Debug, Default, PartialEq, serde::Deserialize, config::Config)]
    #[serde(default)]
    struct AppConfig {
        value: String,
        enabled: bool,
        count: Vec<u32>,
        map: HashMap<String, HashMap<String, Vec<u32>>>,
        non_string_key: HashMap<u32, u32>,
    }

    let source = sources::FileSource::<AppConfig>::toml(
        br#"
    value = "test"
    enabled = true
    count = [1, 2, 3]
    map = { test = { test2 = [1, 2, 3] } }
    non_string_key = { 1 = 2 }
    "#
        .as_slice(),
    )
    .unwrap();

    assert_eq!(
        source.get_key(&"count".into()).unwrap().unwrap(),
        Value::Seq(vec![Value::U32(1), Value::U32(2), Value::U32(3)])
    );
    assert_eq!(
        source.get_key(&"count[0]".into()).unwrap().unwrap(),
        Value::U32(1)
    );
    assert_eq!(
        source.get_key(&"count[1]".into()).unwrap().unwrap(),
        Value::U32(2)
    );
    assert_eq!(
        source.get_key(&"count[2]".into()).unwrap().unwrap(),
        Value::U32(3)
    );
    assert_eq!(source.get_key(&"count[3]".into()).unwrap(), None);

    assert_eq!(
        source
            .get_key(&"map.test.test2[0]".into())
            .unwrap()
            .unwrap(),
        Value::U32(1)
    );
    assert_eq!(
        source
            .get_key(&"map.test.test2[1]".into())
            .unwrap()
            .unwrap(),
        Value::U32(2)
    );
    assert_eq!(
        source
            .get_key(&"map.test.test2[2]".into())
            .unwrap()
            .unwrap(),
        Value::U32(3)
    );
    assert_eq!(source.get_key(&"map.test.test2[3]".into()).unwrap(), None);

    assert_eq!(
        source.get_key(&"non_string_key.1".into()).unwrap().unwrap(),
        Value::U32(2)
    );
}
