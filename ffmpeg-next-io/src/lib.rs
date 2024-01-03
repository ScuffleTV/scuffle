#![doc = include_str!("../README.md")]

mod consts;
mod error;
mod input;
mod output;
mod smart_object;
mod util;

#[cfg(feature = "channel")]
mod channel;

#[cfg(feature = "channel")]
pub use channel::{ChannelCompat, ChannelCompatRecv, ChannelCompatSend};
pub use error::{FfmpegIOError, Result};
pub use input::{Input, InputOptions};
pub use output::{Output, OutputOptions};
