use common::database::{Protobuf, Ulid};

use super::FileType;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UploadedFile {
	pub id: Ulid,
	pub owner_id: Ulid,
	pub uploader_id: Ulid,
	pub name: String,
	#[sqlx(rename = "type")]
	pub ty: FileType,
	pub metadata: Protobuf<pb::scuffle::platform::internal::types::UploadedFileMetadata>,
	pub total_size: i64,
	pub pending: bool,
	pub path: String,
	pub updated_at: chrono::DateTime<chrono::Utc>,
	pub failed: Option<String>,
}
