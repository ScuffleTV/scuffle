use mongodb::bson::oid::ObjectId;
use ulid::Ulid;

use crate::pb::Task;

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct Job {
	#[serde(rename = "_id")]
	pub id: ObjectId,
	pub priority: i32,
	pub hold_until: Option<chrono::DateTime<chrono::Utc>>,
	pub task: Task,
	pub claimed_by_id: Option<ObjectId>,
}
