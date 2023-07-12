use tokio::sync::mpsc;
use web_sys::{Document, HtmlVideoElement, MediaSource};

use crate::player::util::{register_events, Holder};

use super::events::RunnerEvent;

pub fn make_video_holder(
    element: HtmlVideoElement,
    tx: &mpsc::Sender<(RunnerEvent, web_sys::Event)>,
) -> Holder<HtmlVideoElement> {
    let cleanup = register_events!(element, {
        "error" => {
            let tx = tx.clone();
            move |evt| {
                if tx.try_send((RunnerEvent::VideoError, evt)).is_err() {
                    tracing::warn!("Video error event dropped");
                }
            }
        },
        "pause" => {
            let tx = tx.clone();
            move |evt| {
                if tx.try_send((RunnerEvent::VideoPause, evt)).is_err() {
                    tracing::warn!("Video pause event dropped");
                }
            }
        },
            "play" => {
            let tx = tx.clone();
            move |evt| {
                if tx.try_send((RunnerEvent::VideoPlay, evt)).is_err() {
                    tracing::warn!("Video play event dropped");
                }
            }
        },
        "ratechange" => {
            let tx = tx.clone();
            move |evt| {
                if tx.try_send((RunnerEvent::VideoRateChange, evt)).is_err() {
                    tracing::warn!("Video ratechange event dropped");
                }
            }
        },
        "seeked" => {
            let tx = tx.clone();
            move |evt| {
                if tx.try_send((RunnerEvent::VideoSeeked, evt)).is_err() {
                    tracing::warn!("Video seeked event dropped");
                }
            }
        },
        "seeking" => {
            let tx = tx.clone();
            move |evt| {
                if tx.try_send((RunnerEvent::VideoSeeking, evt)).is_err() {
                    tracing::warn!("Video seeking event dropped");
                }
            }
        },
        "stalled" => {
            let tx = tx.clone();
            move |evt| {
                if tx.try_send((RunnerEvent::VideoStalled, evt)).is_err() {
                    tracing::warn!("Video stalled event dropped");
                }
            }
        },
        "suspend" => {
            let tx = tx.clone();
            move |evt| {
                if tx.try_send((RunnerEvent::VideoSuspend, evt)).is_err() {
                    tracing::warn!("Video suspend event dropped");
                }
            }
        },
        "timeupdate" => {
            let tx = tx.clone();
            move |evt| {
                if tx.try_send((RunnerEvent::VideoTimeUpdate, evt)).is_err() {
                    tracing::warn!("Video timeupdate event dropped");
                }
            }
        },
        "volumechange" => {
            let tx = tx.clone();
            move |evt| {
                if tx.try_send((RunnerEvent::VideoVolumeChange, evt)).is_err() {
                    tracing::warn!("Video volumechange event dropped");
                }
            }
        },
        "waiting" => {
            let tx = tx.clone();
            move |evt| {
                if tx.try_send((RunnerEvent::VideoWaiting, evt)).is_err() {
                    tracing::warn!("Video waiting event dropped");
                }
            }
        },
    });

    Holder::new(element, cleanup)
}

pub fn make_media_source_holder(
    media_source: MediaSource,
    tx: &mpsc::Sender<(RunnerEvent, web_sys::Event)>,
) -> Holder<MediaSource> {
    let cleanup = register_events!(media_source, {
        "sourceclose" => {
            let tx = tx.clone();
            move |evt| {
                if tx.try_send((RunnerEvent::MediaSourceClose, evt)).is_err() {
                    tracing::warn!("MediaSource close event dropped")
                }
            }
        },
        "sourceended" => {
            let tx = tx.clone();
            move |evt| {
                if tx.try_send((RunnerEvent::MediaSourceEnded, evt)).is_err() {
                    tracing::warn!("MediaSource ended event dropped")
                }
            }
        },
        "sourceopen" => {
            let tx = tx.clone();
            move |evt| {
                if tx.try_send((RunnerEvent::MediaSourceOpen, evt)).is_err() {
                    tracing::warn!("MediaSource open event dropped")
                }
            }
        },
    });

    Holder::new(media_source, cleanup)
}

pub fn make_document_holder(tx: &mpsc::Sender<(RunnerEvent, web_sys::Event)>) -> Holder<Document> {
    let document = web_sys::window().unwrap().document().unwrap();
    let cleanup = register_events!(document, {
        "visibilitychange" => {
            let tx = tx.clone();
            move |evt| {
                if tx.try_send((RunnerEvent::DocumentVisibilityChange, evt)).is_err() {
                    tracing::warn!("Document visibilitychange event dropped")
                }
            }
        },
    });

    Holder::new(document, cleanup)
}
