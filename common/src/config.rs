use config::ConfigError;
use serde::Deserialize;

// Config Solutions Absolutly Suck
//
// We will have to implement a custom derive macro to properly handle this
// Some minor issues:
//      No config library has support for clap-rs
//      No config library has real hierarchical config support
//      No config library has support for proper env var parsing
//
// Currently we are using config-rs, but it is not ideal.
// config-rs does not support clap and also does not support env var parsing.
// however it is simple and supports multiple config formats.
//

pub fn parse<'de, T: Deserialize<'de>>(config_file: &str) -> Result<T, ConfigError> {
    let mut required = true;
    let config_file = std::env::var("SCUF_CONFIG_FILE").ok().unwrap_or_else(|| {
        required = false;
        config_file.to_string()
    });

    let config = config::Config::builder()
        .add_source(config::File::with_name(&config_file).required(required))
        .add_source(
            config::Environment::with_prefix("SCUF")
                .separator("__")
                .prefix_separator("_"),
        )
        .build()?;

    config.try_deserialize()
}
