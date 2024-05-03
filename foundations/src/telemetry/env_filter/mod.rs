//! This is a direct copy from the `tracing-subscriber` crate.
//! The reason we copied the code is because of https://github.com/tokio-rs/tracing/issues/2951
//! Where the performance of `EnvFilter` is suboptimal.
//! TODO: Remove this once the issue is resolved.

mod directive;
mod env;

pub use env::{Builder as EnvFilterBuilder, EnvFilter};
