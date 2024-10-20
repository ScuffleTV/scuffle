use std::borrow::Cow;
use std::collections::HashMap;
use std::ops::DerefMut;

use anyhow::Context;
use once_cell::sync::OnceCell;
use parking_lot::{RwLock, RwLockWriteGuard};
use prometheus_client::registry::Registry;

use crate::ServiceInfo;

#[doc(hidden)]
pub struct Registries {
	main: RwLock<Registry>,
	optional: RwLock<Registry>,
}

static REGISTRIES: OnceCell<Registries> = OnceCell::new();

impl Registries {
	pub(super) fn init(service_info: ServiceInfo, labels: &HashMap<String, String>) {
		REGISTRIES.get_or_init(|| Registries {
			main: new_registry(
				service_info.metric_name,
				labels.iter().map(|(k, v)| (k.clone().into(), v.clone().into())),
			),
			optional: new_registry(
				service_info.metric_name,
				labels.iter().map(|(k, v)| (k.clone().into(), v.clone().into())),
			),
		});
	}

	pub(super) fn collect(buffer: &mut String, collect_optional: bool) -> anyhow::Result<()> {
		let registries = Self::get();

		if collect_optional {
			encode_registry(&registries.optional.read(), buffer)?;
		}

		encode_registry(&registries.main.read(), buffer)?;

		Ok(())
	}

	pub fn get_main_sub_registry(name: &str) -> impl DerefMut<Target = Registry> {
		let registries = Self::get();
		get_subsystem(registries.main.write(), name)
	}

	pub fn get_optional_sub_registry(name: &str) -> impl DerefMut<Target = Registry> {
		let registries = Self::get();
		get_subsystem(registries.optional.write(), name)
	}

	pub(super) fn get() -> &'static Registries {
		REGISTRIES.get_or_init(|| Registries {
			main: new_registry("", []),
			optional: new_registry("", []),
		})
	}
}

fn new_registry(name: &str, labels: impl IntoIterator<Item = (Cow<'static, str>, Cow<'static, str>)>) -> RwLock<Registry> {
	RwLock::new({
		if name.is_empty() {
			Registry::with_labels(labels.into_iter())
		} else {
			Registry::with_prefix_and_labels(name, labels.into_iter())
		}
	})
}

fn get_subsystem<'a>(registry: RwLockWriteGuard<'a, Registry>, subsystem: &str) -> impl DerefMut<Target = Registry> + 'a {
	RwLockWriteGuard::map(registry, |registry| {
		if subsystem.is_empty() {
			registry
		} else {
			registry.sub_registry_with_prefix(subsystem)
		}
	})
}

fn encode_registry(registry: &Registry, buffer: &mut String) -> anyhow::Result<()> {
	prometheus_client::encoding::text::encode(buffer, registry).context("failed to encode registry")?;

	if buffer.ends_with("# EOF\n") {
		buffer.truncate(buffer.len() - "# EOF\n".len());
	}

	Ok(())
}
