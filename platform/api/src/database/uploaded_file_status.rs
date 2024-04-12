use postgres_types::{FromSql, ToSql};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ToSql, FromSql)]
#[postgres(name = "uploaded_file_status")]
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
