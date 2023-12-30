use common::database::Ulid;

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct FileUploadToken {
	pub id: Ulid,
	pub user_id: Ulid,
	pub max_size: i32,
	pub expires_at: chrono::DateTime<chrono::Utc>,
	// TODO: Some sort of enum which denotes what the file is for and how to handle it after its uploaded. Likely should be
	// a protobuf enum.
}
