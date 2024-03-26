# ⚙️ Config

Once someone said

> Config Solutions Absolutly Suck

This crate tries to solve this problem and make it suck less.

It comes with built-in support for parsing cli arguments, environment variables and files without requiring you to write more than just the definition of your configuration structs.
Another goal of this crate is to be extensible and allow you to write your own sources. You could, for example, implement config sources like databases or remote services.
See [Advanced Usage](#-advanced-usage) for more information.

## 🚀 How to use it

Since this crate is built on top of [serde](https://crates.io/crates/serde), please add serde as a dependency to your `Cargo.toml`.

`cargo add serde --features derive`

After that, you can use the `#[derive(Config)]` macro call to derive the [`Config`](Config) trait for your configuration struct.
All structs implementing [`Config`](Config) are also required to implement [`serde::Deserialize`](::serde::Deserialize).

```rust
#[derive(config::Config, serde::Deserialize)]
struct MyConfig {
    // ...
}
```

Now you can use the [`ConfigBuilder`](ConfigBuilder) to parse your configuration from various sources and merge them together into your configuration struct.

```rust no_run
use config::sources;

#[derive(config::Config, serde::Deserialize)]
struct MyConfig {
    // ...
}

fn main() -> Result<(), config::ConfigError> {
    let mut builder = config::ConfigBuilder::new();
    // From CLI arguments
    builder.add_source(sources::CliSource::new()?);
    // From environment variables
    builder.add_source(sources::EnvSource::with_prefix("TEST")?);
    // From config file
    builder.add_source(sources::FileSource::with_path("config.toml")?);

    // Build the final configuration
    let config: MyConfig = builder.build()?;

    // ...

    Ok(())
}
```

- The first line creates a new [`ConfigBuilder`](ConfigBuilder).
- The next line adds the [`CliSource`](sources::CliSource) to the builder which parses cli arguments using the [`clap`](::clap) crate. You can call your executable with `--help` to see the generated help message.
- The third line adds the [`EnvSource`](sources::EnvSource) to the builder which parses environment variables with the given prefix.
- The third source, called [`FileSource`](sources::FileSource), is added to parse the configuration from a file with the given path. The file format will be detected automatically during runtime. It can be TOML, YAML or JSON.
- Finally, you can call [`build`](ConfigBuilder::build) to build the final configuration. This will parse all sources and merge them together into your configuration struct. The ealier a source is added, the higher its priority is. This means that values from sources added later will **not** overwrite values from sources added earlier.

### Example

See the [examples](./examples) folder for examples.

## 🔧 Advanced usage

### How to define your own config source

To define your own config source, you need to implement the [`Source`](Source) trait.

This requires you to implement the [`get_key`](Source::get_key) method which returns a [`Value`](Value) for a given [`KeyPath`](KeyPath).

That's it. It's as simple as that. Now you can add your source to a [`ConfigBuilder`](ConfigBuilder) and use it to parse your configuration.

## 🔬 How it works under the hood

As soon as your type implements the [`Config`](Config) trait, it supports getting its keys as a [`KeyGraph`](KeyGraph). This is a graph of all keys in that type. It is used by the [`ConfigBuilder`](ConfigBuilder) to retrieve the values from the added sources by iterating all keys.

Please see the docs for [`Config`](Config), [`ConfigBuilder`](ConfigBuilder), [`Source`](Source) and [`KeyGraph`](KeyGraph) for more information.
