use common::database::protobuf;
use ulid::Ulid;

use super::FileType;

#[derive(Debug, Clone, postgres_from_row::FromRow)]
pub struct UploadedFile {
	pub id: Ulid,
	pub owner_id: Ulid,
	pub uploader_id: Ulid,
	pub name: String,
	#[from_row(rename = "type")]
	pub ty: FileType,
	#[from_row(from_fn = "protobuf")]
	pub metadata: pb::scuffle::platform::internal::types::UploadedFileMetadata,
	pub total_size: i64,
	pub pending: bool,
	pub path: String,
	pub updated_at: chrono::DateTime<chrono::Utc>,
	pub failed: Option<String>,
}
