#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunnerEvent {
    VideoError,
    VideoPlay,
    VideoPause,
    VideoSuspend,
    VideoStalled,
    VideoWaiting,
    VideoSeeking,
    VideoSeeked,
    VideoTimeUpdate,
    VideoVolumeChange,
    VideoRateChange,
    MediaSourceOpen,
    MediaSourceClose,
    MediaSourceEnded,
    DocumentVisibilityChange,
}
