//! CLI source
//!
//! ```
//! # use config::sources;
//! #
//! # #[derive(config::Config, serde::Deserialize)]
//! # struct MyConfig {
//! #     // ...
//! # }
//! #
//! # fn main() -> Result<(), config::ConfigError> {
//! let mut builder = config::ConfigBuilder::new();
//! // Add CliSource
//! builder.add_source(sources::CliSource::new()?);
//! // Build the final configuration
//! let config: MyConfig = builder.build()?;
//! # Ok(())
//! # }
//! ```

use std::marker::PhantomData;
use std::sync::Arc;

use clap::{command, Arg, ArgAction, ArgMatches, Command};
use convert_case::{Case, Casing};

use super::utils;
use crate::{Config, ConfigError, ConfigErrorType, ErrorSource, KeyGraph, KeyPath, Result, Source, Value};

fn extend_cmd(
	cmd: Command,
	graph: &KeyGraph,
	arg: Option<Arg>,
	path: &KeyPath,
	sequenced: bool,
) -> Result<(Option<Arg>, Command)> {
	Ok(match graph {
		KeyGraph::String => (Some(arg.unwrap().value_parser(clap::value_parser!(String))), cmd),
		KeyGraph::I8 => (Some(arg.unwrap().value_parser(clap::value_parser!(i8))), cmd),
		KeyGraph::I16 => (Some(arg.unwrap().value_parser(clap::value_parser!(i16))), cmd),
		KeyGraph::I32 => (Some(arg.unwrap().value_parser(clap::value_parser!(i32))), cmd),
		KeyGraph::I64 => (Some(arg.unwrap().value_parser(clap::value_parser!(i64))), cmd),
		KeyGraph::U8 => (Some(arg.unwrap().value_parser(clap::value_parser!(u8))), cmd),
		KeyGraph::U16 => (Some(arg.unwrap().value_parser(clap::value_parser!(u16))), cmd),
		KeyGraph::U32 => (Some(arg.unwrap().value_parser(clap::value_parser!(u32))), cmd),
		KeyGraph::U64 => (Some(arg.unwrap().value_parser(clap::value_parser!(u64))), cmd),
		KeyGraph::F32 => (Some(arg.unwrap().value_parser(clap::value_parser!(f32))), cmd),
		KeyGraph::F64 => (Some(arg.unwrap().value_parser(clap::value_parser!(f64))), cmd),
		KeyGraph::Bool => (
			Some(
				arg.unwrap()
					.value_parser(clap::value_parser!(bool))
					.default_missing_value("true")
					.num_args(0..=1),
			),
			cmd,
		),
		KeyGraph::Unit => {
			if sequenced {
				(
					Some(
						arg.unwrap()
							.value_parser(clap::value_parser!(bool))
							.default_missing_value("true")
							.num_args(0),
					),
					cmd,
				)
			} else {
				(
					Some(arg.unwrap().action(ArgAction::SetTrue).num_args(0).require_equals(false)),
					cmd,
				)
			}
		}
		KeyGraph::Seq(t) => {
			if sequenced {
				return Err(ConfigError::new(ConfigErrorType::UnsupportedType(t.clone())).with_path(path.clone()));
			} else {
				let (arg, cmd) = extend_cmd(cmd, t, arg, path, true)?;
				let Some(arg) = arg else {
					return Err(ConfigError::new(ConfigErrorType::UnsupportedType(t.clone())).with_path(path.clone()));
				};

				let num_args = arg.get_num_args().unwrap();
				(Some(arg.num_args(num_args.min_values()..)), cmd)
			}
		}
		KeyGraph::Option(t) => {
			if sequenced {
				return Err(ConfigError::new(ConfigErrorType::UnsupportedType(t.clone())).with_path(path.clone()));
			}

			let (arg, cmd) = extend_cmd(cmd, t, arg, path, false)?;
			if let Some(arg) = arg {
				let num_args = arg.get_num_args().unwrap();
				(Some(arg.num_args(0..=num_args.max_values())), cmd)
			} else {
				(None, cmd)
			}
		}
		KeyGraph::Struct(map) => {
			if sequenced {
				return Err(
					ConfigError::new(ConfigErrorType::UnsupportedType(Arc::new(graph.clone()))).with_path(path.clone())
				);
			}

			let mut cmd = cmd;

			for (child_path, key) in map {
				if key.skip_cli() {
					continue;
				}

				let path = path.push_struct(child_path);

				let mut arg = Arg::new(path.to_string())
					.long(
						path.iter()
							.map(|v| v.to_string().to_case(Case::Kebab))
							.collect::<Vec<_>>()
							.join("."),
					)
					.num_args(1)
					.required(false);

				if let Some(comment) = key.comment() {
					arg = arg.help(comment);
				}

				let (arg, mut new_cmd) = extend_cmd(cmd, key.graph(), Some(arg), &path, false)?;

				if let Some(arg) = arg {
					new_cmd = new_cmd.arg(arg);
				}

				cmd = new_cmd;
			}

			(None, cmd)
		}
		KeyGraph::Char => (Some(arg.unwrap().value_parser(clap::value_parser!(String))), cmd),
		KeyGraph::Map(_, _) => {
			return Err(ConfigError::new(ConfigErrorType::UnsupportedType(Arc::new(graph.clone()))).with_path(path.clone()));
		}
		KeyGraph::Ref(_, _) => {
			return Err(ConfigError::new(ConfigErrorType::UnsupportedType(Arc::new(graph.clone()))).with_path(path.clone()));
		}
	})
}

pub fn generate_command<C: Config>() -> Result<Command> {
	// Generate clap Command
	let mut command = command!();

	let mut template = "{usage-heading} {usage}\n".to_string();

	if let Some(about) = C::ABOUT {
		let about = about.trim();
		if !about.is_empty() {
			command = command.long_about(about).about(about);
			template += "{about-section}";
		}
	}

	template += "\n{all-args}{tab}";

	if let Some(version) = C::VERSION {
		let version = version.trim();
		if !version.is_empty() {
			command = command.version(version);
		}
	}

	if let Some(author) = C::AUTHOR {
		let author = author.trim();
		if !author.is_empty() {
			command = command.author(author);
			template += "\n\nMaintained by: {author}";
		}
	}

	if let Some(pkg_name) = C::PKG_NAME {
		let pkg_name = pkg_name.trim();
		if !pkg_name.is_empty() {
			command = command.name(pkg_name);
		}
	}

	command = command.help_template(template);

	let graph = C::graph();

	let map = match graph.as_ref() {
		KeyGraph::Struct(map) => map,
		_ => {
			return Err(ConfigError::new(ConfigErrorType::UnsupportedType(graph)).with_path(KeyPath::root()));
		}
	};

	let (arg, mut command) = extend_cmd(command, &KeyGraph::Struct(map.clone()), None, &KeyPath::root(), false)?;

	if let Some(arg) = arg {
		command = command.arg(arg);
	}

	Ok(command)
}

impl From<&KeyPath> for clap::Id {
	fn from(value: &KeyPath) -> Self {
		Self::from(clap::builder::Str::from(value.to_string()))
	}
}

/// Cli source
///
/// Create a new cli source with [`CliSource::new`](CliSource::new) or
/// [`CliSource::with_matches`](CliSource::with_matches).
pub struct CliSource<C: Config> {
	value: Value,
	_phantom: PhantomData<C>,
}

fn matches_to_value(path: KeyPath, graph: &KeyGraph, matches: &ArgMatches, sequenced: bool) -> Result<Option<Value>> {
	let id = path.to_string();

	match graph {
		KeyGraph::Bool => {
			if sequenced {
				Ok(matches
					.get_many::<bool>(&id)
					.map(|s| s.into_iter().map(|s| Value::Bool(*s)).collect())
					.map(Value::Seq))
			} else {
				Ok(matches.get_one::<bool>(&id).map(|s| Value::Bool(*s)))
			}
		}
		KeyGraph::String => {
			if sequenced {
				Ok(matches
					.get_many::<String>(&id)
					.map(|s| s.into_iter().map(|s| Value::String(s.to_string())).collect())
					.map(Value::Seq))
			} else {
				Ok(matches.get_one::<String>(&id).map(|s| Value::String(s.to_string())))
			}
		}
		KeyGraph::I8 => {
			if sequenced {
				Ok(matches
					.get_many::<i8>(&id)
					.map(|s| s.into_iter().map(|s| Value::I8(*s)).collect())
					.map(Value::Seq))
			} else {
				Ok(matches.get_one::<i8>(&id).map(|s| Value::I8(*s)))
			}
		}
		KeyGraph::I16 => {
			if sequenced {
				Ok(matches
					.get_many::<i16>(&id)
					.map(|s| s.into_iter().map(|s| Value::I16(*s)).collect())
					.map(Value::Seq))
			} else {
				Ok(matches.get_one::<i16>(&id).map(|s| Value::I16(*s)))
			}
		}
		KeyGraph::I32 => {
			if sequenced {
				Ok(matches
					.get_many::<i32>(&id)
					.map(|s| s.into_iter().map(|s| Value::I32(*s)).collect())
					.map(Value::Seq))
			} else {
				Ok(matches.get_one::<i32>(&id).map(|s| Value::I32(*s)))
			}
		}
		KeyGraph::I64 => {
			if sequenced {
				Ok(matches
					.get_many::<i64>(&id)
					.map(|s| s.into_iter().map(|s| Value::I64(*s)).collect())
					.map(Value::Seq))
			} else {
				Ok(matches.get_one::<i64>(&id).map(|s| Value::I64(*s)))
			}
		}
		KeyGraph::U8 => {
			if sequenced {
				Ok(matches
					.get_many::<u8>(&id)
					.map(|s| s.into_iter().map(|s| Value::U8(*s)).collect())
					.map(Value::Seq))
			} else {
				Ok(matches.get_one::<u8>(&id).map(|s| Value::U8(*s)))
			}
		}
		KeyGraph::U16 => {
			if sequenced {
				Ok(matches
					.get_many::<u16>(&id)
					.map(|s| s.into_iter().map(|s| Value::U16(*s)).collect())
					.map(Value::Seq))
			} else {
				Ok(matches.get_one::<u16>(&id).map(|s| Value::U16(*s)))
			}
		}
		KeyGraph::U32 => {
			if sequenced {
				Ok(matches
					.get_many::<u32>(&id)
					.map(|s| s.into_iter().map(|s| Value::U32(*s)).collect())
					.map(Value::Seq))
			} else {
				Ok(matches.get_one::<u32>(&id).map(|s| Value::U32(*s)))
			}
		}
		KeyGraph::U64 => {
			if sequenced {
				Ok(matches
					.get_many::<u64>(&id)
					.map(|s| s.into_iter().map(|s| Value::U64(*s)).collect())
					.map(Value::Seq))
			} else {
				Ok(matches.get_one::<u64>(&id).map(|s| Value::U64(*s)))
			}
		}
		KeyGraph::F32 => {
			if sequenced {
				Ok(matches
					.get_many::<f32>(&id)
					.map(|s| s.into_iter().map(|s| Value::F32(*s)).collect())
					.map(Value::Seq))
			} else {
				Ok(matches.get_one::<f32>(&id).map(|s| Value::F32(*s)))
			}
		}
		KeyGraph::F64 => {
			if sequenced {
				Ok(matches
					.get_many::<f64>(&id)
					.map(|s| s.into_iter().map(|s| Value::F64(*s)).collect())
					.map(Value::Seq))
			} else {
				Ok(matches.get_one::<f64>(&id).map(|s| Value::F64(*s)))
			}
		}
		KeyGraph::Unit => {
			if sequenced {
				Ok(matches
					.get_many::<bool>(&id)
					.map(|s| s.into_iter().map(|_| Value::Unit).collect())
					.map(Value::Seq))
			} else {
				Ok(if matches.get_flag(&id) { Some(Value::Unit) } else { None })
			}
		}
		KeyGraph::Seq(t) => {
			if sequenced {
				return Err(ConfigError::new(ConfigErrorType::UnsupportedType(t.clone())).with_path(path));
			}

			matches_to_value(path, t, matches, true)
		}
		KeyGraph::Option(t) => {
			if sequenced {
				return Err(ConfigError::new(ConfigErrorType::UnsupportedType(t.clone())).with_path(path));
			}

			let value = matches_to_value(path, t, matches, false)?;
			if value.is_none() && matches.try_get_raw(&id).map(|v| v.is_some()).unwrap_or_default() {
				Ok(Some(Value::Option(None)))
			} else {
				Ok(value)
			}
		}
		KeyGraph::Struct(map) => {
			if sequenced {
				return Err(ConfigError::new(ConfigErrorType::UnsupportedType(Arc::new(graph.clone()))).with_path(path));
			}

			let mut hashmap = std::collections::BTreeMap::new();

			for (child_path, key) in map {
				if key.skip_cli() {
					continue;
				}

				let path = path.push_struct(child_path);

				let value = matches_to_value(path, key.graph(), matches, false)?;

				if let Some(value) = value {
					hashmap.insert(Value::String(child_path.to_string()), value);
				}
			}

			if hashmap.is_empty() && !path.get_inner().is_empty() {
				Ok(None)
			} else {
				Ok(Some(Value::Map(hashmap)))
			}
		}
		KeyGraph::Char => {
			if sequenced {
				Ok(matches
					.get_many::<String>(&id)
					.map(|s| s.into_iter().map(|s| Value::Char(s.chars().next().unwrap())).collect())
					.map(Value::Seq))
			} else {
				Ok(matches.get_one::<String>(&id).map(|s| Value::Char(s.chars().next().unwrap())))
			}
		}
		KeyGraph::Map(_, _) => {
			Err(ConfigError::new(ConfigErrorType::UnsupportedType(Arc::new(graph.clone()))).with_path(path))
		}
		KeyGraph::Ref(_, _) => {
			Err(ConfigError::new(ConfigErrorType::UnsupportedType(Arc::new(graph.clone()))).with_path(path))
		}
	}
}

impl<C: Config> CliSource<C> {
	/// Creates a new cli source by generating the
	/// [`clap::Command`](::clap::Command) and getting all matches.
	pub fn new() -> Result<Self> {
		Self::with_matches(
			generate_command::<C>()
				.map_err(|e| e.with_source(ErrorSource::Cli))?
				.get_matches(),
		)
	}

	/// Creates a new cli source with given
	/// [`clap::ArgMatches`](::clap::ArgMatches).
	pub fn with_matches(matches: ArgMatches) -> Result<Self> {
		Ok(Self {
			value: matches_to_value(KeyPath::root(), &C::graph(), &matches, false)
				.and_then(|v| C::transform(&KeyPath::root(), v.unwrap_or_else(|| Value::Map(Default::default()))))
				.map_err(|e| e.with_source(ErrorSource::Cli))?,
			_phantom: PhantomData,
		})
	}
}

impl<C: Config> Source<C> for CliSource<C> {
	fn get_key(&self, path: &KeyPath) -> Result<Option<Value>> {
		utils::get_key::<C>(&self.value, path).map_err(|e| e.with_source(ErrorSource::Cli))
	}
}
