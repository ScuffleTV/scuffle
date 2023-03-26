use amf0::Amf0WriteError;

use crate::{chunk::ChunkEncodeError, macros::from_error};
use std::fmt;

#[derive(Debug)]
pub enum NetStreamError {
    Amf0Write(Amf0WriteError),
    ChunkEncode(ChunkEncodeError),
}

from_error!(NetStreamError, Self::Amf0Write, Amf0WriteError);
from_error!(NetStreamError, Self::ChunkEncode, ChunkEncodeError);

impl fmt::Display for NetStreamError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Amf0Write(error) => {
                write!(f, "amf0 write error: {}", error)
            }
            Self::ChunkEncode(error) => write!(f, "chunk encode error: {}", error),
        }
    }
}
