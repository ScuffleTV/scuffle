use file_format::FileFormat;

#[derive(Debug, thiserror::Error)]
pub enum DecoderError {
	#[error("input too long: {0}ms")]
	TooLong(i64),
	#[error("too many frames: {0}frms")]
	TooManyFrames(i64),
	#[error("input too high: {0}px")]
	TooHigh(i32),
	#[error("input too wide: {0}px")]
	TooWide(i32),
	#[error("{0}")]
	Other(anyhow::Error),
}

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

	#[error("publish to nats: {0}")]
	NatsPublish(#[from] async_nats::PublishError),

	#[error("image: {0}")]
	FileFormat(std::io::Error),

	#[error("unsupported input format: {0}")]
	UnsupportedInputFormat(FileFormat),

	#[error("ffmpeg decode: {0}")]
	FfmpegDecode(DecoderError),

	#[error("timelimit exceeded")]
	TimeLimitExceeded,

	#[error("avif decode: {0}")]
	AvifDecode(DecoderError),

	#[error("avif encode: {0}")]
	AvifEncode(anyhow::Error),

	#[error("webp decode: {0}")]
	WebPDecode(DecoderError),

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

	#[error("http download disabled")]
	HttpDownloadDisabled,

	#[error("http download: {0}")]
	HttpDownload(#[from] reqwest::Error),
}

impl ProcessorError {
	pub fn friendly_message(&self) -> String {
		let msg = match self {
			ProcessorError::LostJob => Some("The job was lost"),
			ProcessorError::InvalidJobState => Some("The job is in an invalid state"),
			ProcessorError::DirectoryCreate(_) => Some("Failed to create directory"),
			ProcessorError::FileRead(_) => Some("Failed to read file"),
			ProcessorError::FileCreate(_) => Some("Failed to create file"),
			ProcessorError::S3Download(_) => Some("Failed to download file"),
			ProcessorError::S3Upload(_) => Some("Failed to upload file"),
			ProcessorError::FileFormat(_) => Some("Failed to read file format"),
			ProcessorError::UnsupportedInputFormat(_) => {
				Some("Unsupported input format. Please use one of the supported formats.")
			}
			ProcessorError::TimeLimitExceeded => Some("The job took too long to process the file"),
			ProcessorError::AvifEncode(_) => Some("Failed to reencode image to AVIF"),
			ProcessorError::WebPEncode(_) => Some("Failed to reencode image to WebP"),
			ProcessorError::PngEncode(_) => Some("Failed to reencode image to PNG"),
			ProcessorError::ImageResize(_) => Some("Failed to resize image"),
			ProcessorError::GifskiEncode(_) => Some("Failed to reencode image to GIF"),
			ProcessorError::FfmpegDecode(e) | ProcessorError::AvifDecode(e) | ProcessorError::WebPDecode(e) => match e {
				DecoderError::TooLong(_) => Some("The file is too long"),
				DecoderError::TooManyFrames(_) => Some("The file has too many frames"),
				DecoderError::TooWide(_) => Some("The image is too wide"),
				DecoderError::TooHigh(_) => Some("The image is too high"),
				DecoderError::Other(_) => None,
			},
			_ => None,
		};
		msg.map(|m| m.to_string()).unwrap_or_else(|| format!("{}", self))
	}
}

pub type Result<T, E = ProcessorError> = std::result::Result<T, E>;
