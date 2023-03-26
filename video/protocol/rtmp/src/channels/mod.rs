use bytes::Bytes;
use tokio::sync::{mpsc, oneshot};

pub type UniqueID = uuid::Uuid;

#[derive(Clone, Debug)]
pub enum ChannelData {
    Video { timestamp: u32, data: Bytes },
    Audio { timestamp: u32, data: Bytes },
    MetaData { timestamp: u32, data: Bytes },
}

#[derive(Debug)]
pub struct PublishRequest {
    pub app_name: String,
    pub stream_name: String,
    pub response: oneshot::Sender<UniqueID>,
}

pub type PublishProducer = mpsc::Sender<PublishRequest>;
pub type PublishConsumer = mpsc::Receiver<PublishRequest>;

pub type DataProducer = mpsc::Sender<ChannelData>;
pub type DataConsumer = mpsc::Receiver<ChannelData>;
