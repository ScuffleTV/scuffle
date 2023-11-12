use std::str::FromStr;

use once_cell::sync::OnceCell;
use tracing_subscriber::{prelude::*, reload::Handle, EnvFilter};

static RELOAD_HANDLE: OnceCell<Handle<EnvFilter>> = OnceCell::new();

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Default,
    Json,
    Pretty,
    Compact,
}

#[derive(Debug, thiserror::Error)]
pub enum LoggingError {
    #[error("invalid logging mode: {0}")]
    InvalidMode(String),
    #[error("failed to init logger: {0}")]
    Init(#[from] tracing_subscriber::util::TryInitError),
    #[error("failed to reload logger: {0}")]
    Reload(#[from] tracing_subscriber::reload::Error),
}

pub fn init(level: &str, mode: Mode) -> Result<(), LoggingError> {
    let reload = RELOAD_HANDLE.get_or_try_init(|| {
        let env_filter = EnvFilter::from_str(level).expect("failed to parse log level");

        let filter = tracing_subscriber::fmt()
            .with_line_number(true)
            .with_file(true)
            .with_env_filter(env_filter)
            .with_filter_reloading();

        let handle = filter.reload_handle();

        match mode {
            Mode::Default => filter.finish().try_init(),
            Mode::Json => filter.json().finish().try_init(),
            Mode::Pretty => filter.pretty().finish().try_init(),
            Mode::Compact => filter.compact().finish().try_init(),
        }
        .map(|_| handle)
    })?;

    reload.reload(level)?;

    Ok(())
}
