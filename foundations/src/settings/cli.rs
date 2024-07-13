use anyhow::Context;
use clap::ArgAction;

use super::{Settings, SettingsParser};

const GENERATE_ARG_ID: &str = "generate";
const CONFIG_ARG_ID: &str = "config";

pub use clap;

#[derive(Debug)]
pub struct Cli<S: Settings + serde::de::DeserializeOwned + serde::Serialize> {
	settings: SettingsParser<S>,
	app: clap::Command,
}

fn default_cmd() -> clap::Command {
	clap::Command::new("")
		.arg(
			clap::Arg::new(CONFIG_ARG_ID)
				.long(CONFIG_ARG_ID)
				.short('c')
				.help("The configuration file to use")
				.value_name("FILE")
				.action(ArgAction::Append),
		)
		.arg(
			clap::Arg::new(GENERATE_ARG_ID)
				.long(GENERATE_ARG_ID)
				.help("Generate a configuration file")
				.value_name("FILE")
				.action(ArgAction::Set)
				.num_args(0..=1)
				.default_missing_value("./config.toml"),
		)
}

impl<S: Settings + serde::de::DeserializeOwned + serde::Serialize + Default> Default for Cli<S> {
	fn default() -> Self {
		Self::new(&Default::default())
	}
}

#[derive(Debug, Clone)]
pub struct Matches<S> {
	pub settings: S,
	pub args: clap::ArgMatches,
}

impl<S: Settings + serde::de::DeserializeOwned + serde::Serialize> Cli<S> {
	pub fn new(default: &S) -> Self {
		Self {
			settings: SettingsParser::new(default).unwrap(),
			app: default_cmd(),
		}
	}

	pub fn with_service_info(mut self, info: crate::ServiceInfo) -> Self {
		self.app = self
			.app
			.name(info.name)
			.version(info.version)
			.author(info.author)
			.about(info.description);

		self
	}

	pub fn with_arg(mut self, arg: clap::Arg) -> Self {
		self.app = self.app.arg(arg);
		self
	}

	fn load_file(file: &str, optional: bool) -> anyhow::Result<Option<toml::Value>> {
		let contents = match std::fs::read_to_string(file) {
			Ok(contents) => contents,
			Err(err) => {
				if optional {
					return Ok(None);
				}

				return Err(err).with_context(|| format!("Error reading configuration file: {file}"));
			}
		};

		let incoming = toml::from_str(&contents).with_context(|| format!("Error parsing configuration file: {file}"))?;

		Ok(Some(incoming))
	}

	pub fn parse(mut self) -> anyhow::Result<Matches<S>> {
		let args = self.app.get_matches();

		if let Some(file) = args.get_one::<String>(GENERATE_ARG_ID) {
			let settings = self
				.settings
				.parse()
				.context("failed to construct settings")?
				.to_docs_string()
				.context("failed to serialize settings")?;
			std::fs::write(file, settings).with_context(|| format!("Error writing configuration file: {file}"))?;
			println!("Generated configuration file: {file}");
			std::process::exit(0);
		}

		let mut files = if let Some(files) = args.get_many::<String>(CONFIG_ARG_ID) {
			files.cloned().map(|file| (file, false)).collect::<Vec<_>>()
		} else {
			vec![]
		};

		if files.is_empty() {
			files.push(("config.toml".to_string(), true));
		}

		for (file, optional) in files {
			if let Some(value) = Self::load_file(&file, optional)? {
				self.settings.merge(value);
			}
		}

		Ok(Matches {
			settings: self.settings.parse().context("failed to parse settings")?,
			args,
		})
	}
}
