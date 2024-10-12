use anyhow::Context;
use clap::ArgAction;
use minijinja::syntax::SyntaxConfig;

use super::{Settings, SettingsParser};

const GENERATE_ARG_ID: &str = "generate";
const CONFIG_ARG_ID: &str = "config";
const ALLOW_TEMPLATE: &str = "jinja";

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
		.arg(
			clap::Arg::new(ALLOW_TEMPLATE)
				.long("jinja")
				.help("Allows for the expansion of templates in the configuration file using Jinja syntax")
				.action(ArgAction::Set)
				.num_args(0..=1)
				.default_missing_value("true"),
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

		let mut allow_template = args.get_one::<bool>(ALLOW_TEMPLATE).copied().unwrap_or(true).then(|| {
			let mut env = minijinja::Environment::new();

			env.add_global("env", std::env::vars().collect::<std::collections::HashMap<_, _>>());
			env.set_syntax(
				SyntaxConfig::builder()
					.block_delimiters("{%", "%}")
					.variable_delimiters("${{", "}}")
					.comment_delimiters("{#", "#}")
					.build()
					.unwrap(),
			);

			env
		});

		let mut files = if let Some(files) = args.get_many::<String>(CONFIG_ARG_ID) {
			files.cloned().map(|file| (file, false)).collect::<Vec<_>>()
		} else {
			vec![]
		};

		if files.is_empty() {
			files.push(("config.toml".to_string(), true));
		}

		for (file, optional) in files {
			let content = match std::fs::read_to_string(file) {
				Ok(content) => content,
				Err(err) => {
					if optional && err.kind() == std::io::ErrorKind::NotFound {
						continue;
					}

					return Err(err).context("read");
				}
			};

			let content = if let Some(env) = &mut allow_template {
				env.template_from_str(&content)
					.context("template")?
					.render(())
					.context("render")?
			} else {
				content
			};

			let incoming = toml::from_str(&content).context("parse")?;
			self.settings.merge(incoming);
		}

		Ok(Matches {
			settings: self.settings.parse().context("failed to parse settings")?,
			args,
		})
	}
}
