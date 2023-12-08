use file_format::FileFormat;

#[derive(Debug, thiserror::Error)]
pub enum ProcessorError {
	#[error("semaphore ticket acquire: {0}")]
	SemaphoreAcquire(#[from] tokio::sync::AcquireError),

	#[error("sqlx: {0}")]
	Sqlx(#[from] sqlx::Error),

	#[error("lost job")]
	LostJob,

	#[error("invalid job state")]
	InvalidJobState,

	#[error("directory create: {0}")]
	DirectoryCreate(std::io::Error),

	#[error("file read: {0}")]
	FileRead(std::io::Error),

	#[error("working directory change: {0}")]
	WorkingDirectoryChange(std::io::Error),

	#[error("file create: {0}")]
	FileCreate(std::io::Error),

	#[error("download source from s3: {0}")]
	S3Download(s3::error::S3Error),

	#[error("upload target to s3: {0}")]
	S3Upload(s3::error::S3Error),

	#[error("image: {0}")]
	FileFormat(std::io::Error),

	#[error("unsupported input format: {0}")]
	UnsupportedInputFormat(FileFormat),

	#[error("ffmpeg decode: {0}")]
	FfmpegDecode(anyhow::Error),

	#[error("timelimit exceeded")]
	TimeLimitExceeded,

	#[error("avif decode: {0}")]
	AvifDecode(anyhow::Error),

	#[error("avif encode: {0}")]
	AvifEncode(anyhow::Error),

	#[error("webp decode: {0}")]
	WebPDecode(anyhow::Error),

	#[error("webp encode: {0}")]
	WebPEncode(anyhow::Error),

	#[error("png encode: {0}")]
	PngEncode(anyhow::Error),

	#[error("image resize: {0}")]
	ImageResize(anyhow::Error),

	#[error("blocking task spawn")]
	BlockingTaskSpawn,

	#[error("gifski encode: {0}")]
	GifskiEncode(anyhow::Error),
}

pub type Result<T, E = ProcessorError> = std::result::Result<T, E>;
