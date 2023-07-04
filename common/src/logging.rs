use std::str::FromStr;

use anyhow::Result;
use once_cell::sync::OnceCell;
use tracing_subscriber::{prelude::*, reload::Handle, EnvFilter};

static RELOAD_HANDLE: OnceCell<Handle<EnvFilter>> = OnceCell::new();

pub fn init(level: &str, json: bool) -> Result<()> {
    let reload = RELOAD_HANDLE.get_or_try_init(|| {
        let env_filter = EnvFilter::from_str(level).expect("failed to parse log level");

        let filter = tracing_subscriber::fmt()
            .with_line_number(true)
            .with_file(true)
            .with_env_filter(env_filter)
            .with_filter_reloading();

        let handle = filter.reload_handle();

        if json {
            filter.json().finish().try_init()
        } else {
            filter.pretty().finish().try_init()
        }
        .map(|_| handle)
    })?;

    reload.reload(level)?;

    Ok(())
}
