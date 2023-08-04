use chrono::{DateTime, Utc};
use pb::scuffle::internal::video::types::StreamState;
use uuid::Uuid;

use super::protobuf::ProtobufValue;

#[derive(Debug, Clone, Default, Copy, Eq, PartialEq)]
#[repr(i64)]
pub enum ReadyState {
    #[default]
    NotReady = 0,
    Ready = 1,
    Stopped = 2,
    StoppedResumable = 3,
    Failed = 4,
    WasReady = 5,
}

impl From<ReadyState> for i64 {
    fn from(state: ReadyState) -> Self {
        match state {
            ReadyState::NotReady => 0,
            ReadyState::Ready => 1,
            ReadyState::Stopped => 2,
            ReadyState::StoppedResumable => 3,
            ReadyState::Failed => 4,
            ReadyState::WasReady => 5,
        }
    }
}

impl From<i64> for ReadyState {
    fn from(state: i64) -> Self {
        match state {
            0 => ReadyState::NotReady,
            1 => ReadyState::Ready,
            2 => ReadyState::Stopped,
            3 => ReadyState::StoppedResumable,
            4 => ReadyState::Failed,
            5 => ReadyState::WasReady,
            _ => ReadyState::NotReady,
        }
    }
}

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct Model {
    /// The unique identifier for the stream.
    pub id: Uuid,
    /// The unique identifier for the channel which owns the stream.
    pub channel_id: Uuid,
    /// The current title of the stream.
    pub title: String,
    /// The current description of the stream.
    pub description: String,
    /// Whether or not the stream had recording enabled.
    pub recorded: bool,
    /// Whether or not the stream had transcoding enabled.
    pub transcoded: bool,
    /// Whether or not the stream has been deleted.
    pub deleted: bool,
    /// Whether or not the stream is ready to be viewed.
    pub ready_state: ReadyState,
    /// Ingest Address address of the ingest server controlling the stream.
    pub ingest_address: String,
    /// The connection which owns the stream.
    pub connection_id: Uuid,
    /// The Stream Variants
    pub state: ProtobufValue<StreamState>,
    /// The time the stream was created.
    pub created_at: DateTime<Utc>,
    /// The time the stream was last updated.
    /// Used to check if the stream is alive or if its resumable.
    pub updated_at: Option<DateTime<Utc>>,
    /// The time the stream ended. (will be in the future if the stream is live)
    pub ended_at: DateTime<Utc>,
}
