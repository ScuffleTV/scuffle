use std::sync::Arc;

use bson::Bson;
use mongodb::{bson::oid::ObjectId, Database, IndexModel};
use scuffle_image_processor_proto::Task;
use serde::{Deserialize, Serializer};

use crate::global::Global;

fn serialize_protobuf<T: prost::Message, S: Serializer>(value: &T, serializer: S) -> Result<S::Ok, S::Error> {
	serializer.serialize_bytes(&value.encode_to_vec())
}

fn deserialize_protobuf<'de, T: prost::Message + Default, D: serde::Deserializer<'de>>(deserializer: D) -> Result<T, D::Error> {
	let bytes = Vec::<u8>::deserialize(deserializer)?;
	T::decode(bytes.as_slice()).map_err(serde::de::Error::custom)
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct Job {
	#[serde(rename = "_id")]
	pub id: ObjectId,
	pub priority: i32,
	pub hold_until: Option<chrono::DateTime<chrono::Utc>>,
	#[serde(serialize_with = "serialize_protobuf", deserialize_with = "deserialize_protobuf")]
	pub task: Task,
	pub claimed_by_id: Option<ObjectId>,
}

impl Job {
	fn collection(database: &Database) -> mongodb::Collection<Job> {
		database.collection("jobs")
	}

	pub async fn setup_collection(database: &Database) -> Result<(), mongodb::error::Error> {
		let collection = Self::collection(database);

		collection.create_index(
			IndexModel::builder()
				.keys(bson::doc! {
					"hold_until": 1,
					"priority": -1,
				})
				.build(),
			None,
		).await?;

		Ok(())
	}

	pub async fn new(global: &Arc<Global>, task: Task, priority: i32) -> Result<Self, mongodb::error::Error> {
		let job = Job {
			id: ObjectId::new(),
			priority,
			hold_until: None,
			task,
			claimed_by_id: None,
		};

		Self::collection(global.database()).insert_one(&job, None).await?;

		Ok(job)
	}

	pub async fn fetch(global: &Arc<Global>) -> Result<Option<Self>, mongodb::error::Error> {
		// Find with the highest priority and no hold_until or hold_until in the past
		Self::collection(global.database())
			.find_one_and_update(
				bson::doc! {
					"$or": [
						bson::doc! {
							"hold_until": Bson::Null,
						},
						bson::doc! {
							"hold_until": {
								"$lt": chrono::Utc::now(),
							},
						},
					],
				},
				bson::doc! {
					"$set": {
						"claimed_by_id": global.worker_id(),
						"hold_until": chrono::Utc::now() + chrono::Duration::seconds(60),
					},
				},
				Some(
					mongodb::options::FindOneAndUpdateOptions::builder()
						.sort(bson::doc! {
							"priority": -1,
						})
						.build(),
				),
			)
			.await
	}

	pub async fn refresh(&self, global: &Arc<Global>) -> Result<bool, mongodb::error::Error> {
		let success = Self::collection(global.database())
			.update_one(
				bson::doc! {
					"_id": self.id.clone(),
					"claimed_by_id": global.worker_id(),
				},
				bson::doc! {
					"$set": {
						"hold_until": chrono::Utc::now() + chrono::Duration::seconds(60),
					},
				},
				None,
			)
			.await?;

		Ok(success.modified_count == 1)
	}

	pub async fn complete(&self, global: &Arc<Global>) -> Result<bool, mongodb::error::Error> {
		let success = Self::collection(global.database())
			.delete_one(
				bson::doc! {
					"_id": self.id.clone(),
					"claimed_by_id": global.worker_id(),
				},
				None,
			)
			.await?;

		Ok(success.deleted_count == 1)
	}
}
