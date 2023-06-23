use tokio::sync::mpsc;
use wasm_bindgen::JsValue;
use web_sys::{MediaSource, SourceBuffer, TimeRanges};

use crate::player::util::{register_events, Holder};

pub struct SourceBufferHolder {
    sb: Holder<SourceBuffer>,
    rx: mpsc::Receiver<()>,
}

impl SourceBufferHolder {
    pub fn new(media_source: &MediaSource, codec: &str) -> Result<Self, JsValue> {
        let sb = media_source.add_source_buffer(codec)?;
        let (tx, rx) = mpsc::channel(128);

        let cleanup = register_events!(sb, {
            "updateend" => move |_| {
                if tx.try_send(()).is_err() {
                    tracing::warn!("failed to send updateend event");
                }
            }
        });

        Ok(Self {
            sb: Holder::new(sb, cleanup),
            rx,
        })
    }

    pub fn buffered(&self) -> Result<TimeRanges, JsValue> {
        self.sb.buffered()
    }

    pub fn change_type(&self, codec: &str) -> Result<(), JsValue> {
        self.sb.change_type(codec)?;
        Ok(())
    }

    pub async fn append_buffer(&mut self, mut data: Vec<u8>) -> Result<(), JsValue> {
        self.sb.append_buffer_with_u8_array(data.as_mut_slice())?;
        self.rx.recv().await;
        Ok(())
    }

    pub async fn remove(&mut self, start: f64, end: f64) -> Result<(), JsValue> {
        if start >= end {
            return Ok(());
        }

        self.sb.remove(start, end)?;
        self.rx.recv().await;
        Ok(())
    }
}

pub enum SourceBuffers {
    AudioVideoSplit {
        audio: SourceBufferHolder,
        video: SourceBufferHolder,
    },
    None,
    AudioVideoCombined(SourceBufferHolder),
}

impl SourceBuffers {
    pub fn audio(&mut self) -> Option<&mut SourceBufferHolder> {
        match self {
            Self::AudioVideoSplit { audio, .. } => Some(audio),
            _ => None,
        }
    }

    pub fn video(&mut self) -> Option<&mut SourceBufferHolder> {
        match self {
            Self::AudioVideoSplit { video, .. } => Some(video),
            _ => None,
        }
    }

    pub fn audiovideo(&mut self) -> Option<&mut SourceBufferHolder> {
        match self {
            Self::AudioVideoCombined(audiovideo) => Some(audiovideo),
            _ => None,
        }
    }
}
