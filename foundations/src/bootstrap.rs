use std::future::Future;

use anyhow::Context;
use scuffle_foundations_macros::auto_settings;

pub use scuffle_foundations_macros::bootstrap;

use crate::{
    settings::{
        cli::{Cli, Matches},
        Settings,
    },
    BootstrapResult,
};

pub fn bootstrap<
    C: Bootstrap + std::fmt::Debug,
    F: Fn(Matches<C>) -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
>(
    config: &C,
    info: crate::ServiceInfo,
    main: F,
) -> BootstrapResult<()> {
    let mut cli = Cli::<C>::new(config).with_service_info(info);

    for arg in C::additional_args() {
        cli = cli.with_arg(arg);
    }

    let matches = cli.parse()?;

    let runtime = match matches.settings.runtime_mode() {
        RuntimeSettings::Steal { name, threads } => {
            crate::runtime::Runtime::new_steal(threads, &name)
        }
        RuntimeSettings::NoSteal { name, threads } => {
            crate::runtime::Runtime::new_no_steal(threads, &name)
        }
    }
    .context("Failed to create runtime")?;

    runtime.block_on(async move {
        #[cfg(feature = "_telemetry")]
        if let Some(telemetry) = matches.settings.telemetry_config() {
            crate::telementry::settings::init(info, telemetry).await;
        }

        main(matches).await
    })
}

#[auto_settings(crate_path = "crate")]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum RuntimeSettings {
    Steal {
        threads: usize,
        name: String,
    },
    #[settings(default)]
    NoSteal {
        threads: usize,
        name: String,
    },
}

pub trait Bootstrap: serde::Serialize + serde::de::DeserializeOwned + Settings {
    fn runtime_mode(&self) -> RuntimeSettings {
        RuntimeSettings::NoSteal {
            threads: num_cpus::get(),
            name: String::new(),
        }
    }

    #[cfg(feature = "_telemetry")]
    fn telemetry_config(&self) -> Option<crate::telementry::settings::TelementrySettings> {
        None
    }

    fn additional_args() -> Vec<clap::Arg> {
        vec![]
    }
}
