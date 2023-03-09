use async_graphql::SimpleObject;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(SimpleObject, Deserialize, Serialize)]
pub struct Message {
    #[graphql(skip)]
    pub chat_id: i64,
    pub username: String,
    pub content: String,
    pub metadata: HashMap<String, String>,
    pub message_type: String,
}
