#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub enum FfmpegIOError {
	#[error("null pointer: {0}")]
	NullPointer(&'static str),

	#[error("ffmpeg error: {0}")]
	Ffmpeg(#[from] ffmpeg_next::Error),

	#[error("arguments: {0}")]
	Arguments(&'static str),
}

pub type Result<T, E = FfmpegIOError> = std::result::Result<T, E>;

pub(crate) type ResponseResult<T, S> = std::result::Result<T, (S, FfmpegIOError)>;
