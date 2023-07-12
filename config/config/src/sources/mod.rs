//! All built-in configuration sources

pub mod cli;
pub mod env;
pub mod file;
pub mod manual;

mod utils;

pub use cli::CliSource;
pub use env::EnvSource;
pub use file::FileSource;
pub use manual::ManualSource;
