#![allow(dead_code)]
// TODO: Remove this once we have a real implementation

use crate::{global::GlobalState, pb::scuffle::video::transcoder_server};
use std::sync::{Arc, Weak};

use tonic::{async_trait, Status};

pub struct TranscoderServer {
    global: Weak<GlobalState>,
}

impl TranscoderServer {
    pub fn new(global: &Arc<GlobalState>) -> Self {
        Self {
            global: Arc::downgrade(global),
        }
    }
}

type Result<T> = std::result::Result<T, Status>;

#[async_trait]
impl transcoder_server::Transcoder for TranscoderServer {}
