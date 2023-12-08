use common::database::{Protobuf, Ulid};
use pb::scuffle::platform::internal::image_processor::Task;

// The actual table has more columns but we only need id and task to process a
// job

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct Job {
	pub id: Ulid,
	pub task: Protobuf<Task>,
}
