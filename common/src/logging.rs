use std::{str::FromStr, sync::RwLock};

use anyhow::Result;
use tracing_subscriber::{
    prelude::*,
    reload::{self, Handle},
    EnvFilter, Layer, Registry,
};

type HandleType = Handle<EnvFilter, Registry>;

static ONCE: std::sync::Once = std::sync::Once::new();
static RELOAD_HANDLE: RwLock<Option<HandleType>> = RwLock::new(None);

pub fn init(level: &str, json: bool) -> Result<()> {
    let mut result: Result<(), anyhow::Error> = Ok(());
    ONCE.call_once(|| {
        let (env_filter, handle) =
            reload::Layer::new(EnvFilter::from_str(level).expect("failed to parse log level"));

        let filter = tracing_subscriber::fmt::layer()
            .with_line_number(true)
            .with_file(true);

        if json {
            let filter = filter.json().with_filter(env_filter);

            let registry = tracing_subscriber::registry().with(filter);

            result = tracing::subscriber::set_global_default(registry).map_err(|e| e.into());
            if result.is_err() {
                return;
            }
        } else {
            let filter = filter.with_filter(env_filter);

            let registry = tracing_subscriber::registry().with(filter);

            result = tracing::subscriber::set_global_default(registry).map_err(|e| e.into());
            if result.is_err() {
                return;
            }
        }

        *RELOAD_HANDLE.write().expect("failed to write to handler") = Some(handle);
    });

    result?;

    RELOAD_HANDLE
        .read()
        .expect("failed to read mutex")
        .as_ref()
        .expect("failed to get reload handle")
        .reload(level)?;

    Ok(())
}
