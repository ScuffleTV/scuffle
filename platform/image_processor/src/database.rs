use common::database::protobuf;
use pb::scuffle::platform::internal::image_processor::Task;
use ulid::Ulid;

// The actual table has more columns but we only need id and task to process a
// job

#[derive(Debug, Clone, Default, postgres_from_row::FromRow)]
pub struct Job {
	pub id: Ulid,
	#[from_row(from_fn = "protobuf")]
	pub task: Task,
}
