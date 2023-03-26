mod define;
mod errors;
mod reader;
mod writer;

pub use crate::{
    define::{Amf0Marker, Amf0Value},
    errors::{Amf0ReadError, Amf0WriteError},
    reader::Amf0Reader,
    writer::Amf0Writer,
};

#[cfg(test)]
mod tests;
