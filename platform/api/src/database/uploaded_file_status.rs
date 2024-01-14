use postgres_types::{ToSql, FromSql};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ToSql, FromSql)]
pub enum UploadedFileStatus {
	#[postgres(name = "unqueued")]
	Unqueued,
	#[postgres(name = "queued")]
	Queued,
	#[postgres(name = "failed")]
	Failed,
	#[postgres(name = "completed")]
	Completed,
}
