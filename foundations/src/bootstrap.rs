use std::future::Future;

use anyhow::Context;
use scuffle_foundations_macros::auto_settings;
pub use scuffle_foundations_macros::bootstrap;

use crate::settings::cli::{Cli, Matches};
use crate::settings::Settings;
use crate::BootstrapResult;

pub fn bootstrap<C: Bootstrap, F: Fn(Matches<C>) -> Fut, Fut: Future<Output = anyhow::Result<()>>>(
	default_settings: &C::Settings,
	info: crate::ServiceInfo,
	main: F,
) -> BootstrapResult<()> {
	let mut cli = Cli::<C::Settings>::new(default_settings).with_service_info(info);

	for arg in C::additional_args() {
		cli = cli.with_arg(arg);
	}

	let matches = cli.parse()?;

	let matches = Matches {
		settings: C::from(matches.settings),
		args: matches.args,
	};

	let runtime = match matches.settings.runtime_mode() {
		RuntimeSettings::Steal { name, threads } => crate::runtime::Runtime::new_steal(threads, &name),
		RuntimeSettings::NoSteal { name, threads } => crate::runtime::Runtime::new_no_steal(threads, &name),
	}
	.context("Failed to create runtime")?;

	runtime.block_on(async move {
		#[cfg(feature = "_telemetry")]
		if let Some(telemetry) = matches.settings.telemetry_config() {
			crate::telemetry::settings::init(info, telemetry).await;
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

pub trait Bootstrap: Sized + From<Self::Settings> {
	type Settings: serde::Serialize + serde::de::DeserializeOwned + Settings;

	fn runtime_mode(&self) -> RuntimeSettings {
		RuntimeSettings::NoSteal {
			threads: num_cpus::get(),
			name: String::new(),
		}
	}

	#[cfg(feature = "_telemetry")]
	fn telemetry_config(&self) -> Option<crate::telemetry::settings::TelemetrySettings> {
		None
	}

	fn additional_args() -> Vec<clap::Arg> {
		vec![]
	}
}

impl Bootstrap for () {
	type Settings = Self;
}
