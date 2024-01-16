use std::str::FromStr;

use once_cell::sync::OnceCell;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

type ReloadHandle = Box<dyn Fn(&str) -> Result<(), LoggingError> + Sync + Send>;

static RELOAD_HANDLE: OnceCell<ReloadHandle> = OnceCell::new();

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
	InvalidMode(#[from] tracing_subscriber::filter::ParseError),
	#[error("failed to init logger: {0}")]
	Init(#[from] tracing_subscriber::util::TryInitError),
	#[error("failed to reload logger: {0}")]
	Reload(#[from] tracing_subscriber::reload::Error),
}

pub fn init(level: &str, mode: Mode) -> Result<(), LoggingError> {
	let reload = RELOAD_HANDLE.get_or_try_init(|| {
		let env_filter = EnvFilter::from_str(level)?;

		match mode {
			Mode::Default => {
				let filter = tracing_subscriber::fmt()
					.with_line_number(true)
					.with_file(true)
					.with_env_filter(env_filter)
					.with_filter_reloading();

				let handle = filter.reload_handle();

				filter.finish().try_init()?;

				Ok::<_, LoggingError>(Box::new(move |level: &str| {
					let level = EnvFilter::from_str(level)?;
					handle.reload(level)?;
					Ok(())
				}) as ReloadHandle)
			}
			Mode::Json => {
				let filter = tracing_subscriber::fmt()
					.json()
					.with_line_number(true)
					.with_file(true)
					.with_env_filter(env_filter)
					.with_filter_reloading();

				let handle = filter.reload_handle();

				filter.finish().try_init()?;

				Ok(Box::new(move |level: &str| {
					let level = EnvFilter::from_str(level)?;
					handle.reload(level)?;
					Ok(())
				}) as ReloadHandle)
			}
			Mode::Pretty => {
				let filter = tracing_subscriber::fmt()
					.pretty()
					.with_line_number(true)
					.with_file(true)
					.with_env_filter(env_filter)
					.with_filter_reloading();

				let handle = filter.reload_handle();

				filter.finish().try_init()?;

				Ok(Box::new(move |level: &str| {
					let level = EnvFilter::from_str(level)?;
					handle.reload(level)?;
					Ok(())
				}) as ReloadHandle)
			}
			Mode::Compact => {
				let filter = tracing_subscriber::fmt()
					.compact()
					.with_line_number(true)
					.with_file(true)
					.with_env_filter(env_filter)
					.with_filter_reloading();

				let handle = filter.reload_handle();

				filter.finish().try_init()?;

				Ok(Box::new(move |level: &str| {
					let level = EnvFilter::from_str(level)?;
					handle.reload(level)?;
					Ok(())
				}) as ReloadHandle)
			}
		}
	})?;

	reload(level)?;

	Ok(())
}
