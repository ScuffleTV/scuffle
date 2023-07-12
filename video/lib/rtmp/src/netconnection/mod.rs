mod errors;
mod writer;

pub use self::{errors::NetConnectionError, writer::NetConnection};

#[cfg(test)]
mod tests;
