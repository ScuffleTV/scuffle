use async_graphql::SimpleObject;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(SimpleObject, Deserialize)]
pub struct Message {
    #[graphql(skip)]
    pub chat_id: i64,
    pub username: Option<String>,
    pub content: String,
    pub metadata: Option<HashMap<String, String>>,
    pub message_type: Option<String>,
}
