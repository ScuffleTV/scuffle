//! Run with: `cargo run --example derive`
//! Look at the generated code with: `cargo expand --example derive`

use config::{sources, Config, ConfigBuilder, ConfigError};

type TypeAlias = bool;

#[derive(config::Config, Debug, PartialEq, serde::Deserialize, Default)]
#[serde(default)]
struct AppConfig {
	enabled: TypeAlias,
	logging: LoggingConfig,
	#[config(cli(skip), env(skip))]
	count: Vec<Vec<u8>>,
}

#[derive(config::Config, Debug, PartialEq, serde::Deserialize)]
#[serde(default)]
struct LoggingConfig {
	level: String,
	json: bool,
}

impl Default for LoggingConfig {
	fn default() -> Self {
		Self {
			level: "INFO".to_string(),
			json: false,
		}
	}
}

fn main() {
	match parse() {
		Ok(config) => println!("{:#?}", config),
		Err(err) => println!("{:#}", err),
	}
}

fn parse() -> Result<AppConfig, ConfigError> {
	dbg!(AppConfig::graph());

	let mut builder = ConfigBuilder::new();
	builder.add_source(sources::CliSource::new()?);
	builder.add_source(sources::EnvSource::with_prefix("TEST")?);
	builder.add_source(sources::FileSource::json(
		br#"
    {
        "enabled": "on",
        "count": [[2], [122]],
        "logging": {
            "level": "DEBUG",
            "json": true
        }
    }
    "#
		.as_slice(),
	)?);

	builder.overwrite("logging.level", "TEST")?;
	builder.overwrite("logging.json", "off")?;

	let config: AppConfig = builder.build()?;

	Ok(config)
}
