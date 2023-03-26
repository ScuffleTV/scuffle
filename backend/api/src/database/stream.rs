use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Default, Copy, Eq, PartialEq)]
#[repr(i64)]
pub enum State {
    #[default]
    NotReady = 0,
    Ready = 1,
    Stopped = 2,
    StoppedResumable = 3,
    Failed = 4,
    WasReady = 5,
}

impl From<State> for i64 {
    fn from(state: State) -> Self {
        match state {
            State::NotReady => 0,
            State::Ready => 1,
            State::Stopped => 2,
            State::StoppedResumable => 3,
            State::Failed => 4,
            State::WasReady => 5,
        }
    }
}

impl From<i64> for State {
    fn from(state: i64) -> Self {
        match state {
            0 => State::NotReady,
            1 => State::Ready,
            2 => State::Stopped,
            3 => State::StoppedResumable,
            4 => State::Failed,
            5 => State::WasReady,
            _ => State::NotReady,
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
    pub state: State,
    /// Ingest Address address of the ingest server controlling the stream.
    pub ingest_address: String,
    /// The connection which owns the stream.
    pub connection_id: Uuid,
    /// The time the stream was created.
    pub created_at: DateTime<Utc>,
    /// The time the stream was last updated.
    /// Used to check if the stream is alive or if its resumable.
    pub updated_at: Option<DateTime<Utc>>,
    /// The time the stream ended. (will be in the future if the stream is live)
    pub ended_at: DateTime<Utc>,
}
