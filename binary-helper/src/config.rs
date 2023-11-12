use common::config::{DatabaseConfig, GrpcConfig, LoggingConfig, NatsConfig};

use super::Config;

#[derive(Debug, Clone, PartialEq, config::Config, serde::Deserialize)]
#[serde(default)]
pub struct AppConfig<T: ConfigExtention> {
    /// The name of the application
    pub name: String,

    /// The path to the config file
    pub config_file: Option<String>,

    /// The logging configuration
    pub logging: LoggingConfig,

    /// The gRPC configuration
    pub grpc: GrpcConfig,

    /// The database configuration
    pub database: DatabaseConfig,

    /// The NATS configuration
    pub nats: NatsConfig,

    #[serde(flatten)]
    #[config(flatten)]
    pub extra: T,
}

pub trait ConfigExtention: config::Config + Default {
    const APP_NAME: &'static str;

    fn config_default() -> AppConfig<Self> {
        AppConfig {
            name: Self::APP_NAME.to_owned(),
            config_file: Some("config".to_owned()),
            logging: Default::default(),
            grpc: Default::default(),
            database: Default::default(),
            nats: Default::default(),
            extra: Self::default(),
        }
    }

    fn pre_hook(_config: &mut AppConfig<Self>) -> anyhow::Result<()> {
        Ok(())
    }
}

impl<T: ConfigExtention> Default for AppConfig<T> {
    fn default() -> Self {
        T::config_default()
    }
}

impl<'de, T: ConfigExtention + serde::Deserialize<'de>> Config for AppConfig<T> {
    fn logging(&self) -> &LoggingConfig {
        &self.logging
    }

    fn parse() -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let (mut config, config_file) =
            common::config::parse::<Self>(!cfg!(test), Self::default().config_file)?;

        config.config_file = config_file;

        Ok(config)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn pre_hook(&mut self) -> anyhow::Result<()> {
        T::pre_hook(self)
    }
}
