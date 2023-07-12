use async_graphql::SimpleObject;
use uuid::Uuid;

use crate::database::stream;

use super::date::DateRFC3339;

#[derive(SimpleObject, Clone)]
pub struct Stream {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub title: String,
    pub description: String,
    pub recorded: bool,
    pub transcoded: bool,
    pub deleted: bool,
    pub ready_state: i64,
    pub created_at: DateRFC3339,
    pub ended_at: DateRFC3339,
}

impl From<stream::Model> for Stream {
    fn from(value: stream::Model) -> Self {
        Self {
            id: value.id,
            channel_id: value.channel_id,
            title: value.title,
            description: value.description,
            recorded: value.recorded,
            transcoded: value.transcoded,
            deleted: value.deleted,
            ready_state: value.ready_state.into(),
            created_at: value.created_at.into(),
            ended_at: value.ended_at.into(),
        }
    }
}
