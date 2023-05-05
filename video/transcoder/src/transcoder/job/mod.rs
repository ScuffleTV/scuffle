use std::io;
use std::process::Output;
use std::{
    os::unix::process::CommandExt, path::Path, pin::pin, process::Command as StdCommand, sync::Arc,
    time::Duration,
};

use anyhow::{anyhow, Result};
use common::prelude::*;
use common::vec_of_strings;
use fred::types::Expiration;
use futures::{stream::FuturesUnordered, FutureExt, StreamExt};
use lapin::message::Delivery;
use lapin::options::BasicAckOptions;
use nix::sys::signal;
use nix::unistd::Pid;
use prost::Message as _;
use tokio::{
    io::AsyncWriteExt,
    join,
    net::UnixListener,
    process::{ChildStdin, Command},
    select,
};
use tokio_util::sync::CancellationToken;
use tonic::{transport::Channel, Status};

use crate::pb::scuffle::types::StreamVariant;
use crate::transcoder::job::utils::{SharedFuture, set_lock, release_lock};
use crate::transcoder::job::variant::make_audio_stream;
use crate::{
    global::GlobalState,
    pb::scuffle::{
        events::{transcoder_message, TranscoderMessage, TranscoderMessageNewStream},
        video::{
            ingest_client::IngestClient, transcoder_event_request, watch_stream_response,
            TranscoderEventRequest, WatchStreamRequest, WatchStreamResponse,
        },
    },
    transcoder::job::{utils::MultiStream, variant::TrackSetup},
};
use fred::interfaces::KeysInterface;

mod track_parser;
mod utils;
mod variant;

pub async fn handle_message(
    global: Arc<GlobalState>,
    msg: Delivery,
    shutdown_token: CancellationToken,
) {
    let mut job = match handle_message_internal(&msg).await {
        Ok(job) => job,
        Err(err) => {
            tracing::error!("failed to handle message: {}", err);
            return;
        }
    };

    if let Err(err) = msg.ack(BasicAckOptions::default()).await {
        tracing::error!("failed to ACK message: {}", err);
        return;
    };

    job.run(global, shutdown_token).await;
}

async fn handle_message_internal(msg: &Delivery) -> Result<Job> {
    let message = TranscoderMessage::decode(msg.data.as_slice())?;

    let req = match message.data {
        Some(transcoder_message::Data::NewStream(data)) => data,
        None => return Err(anyhow!("message missing data")),
    };

    let channel = common::grpc::make_channel(
        vec![req.ingest_address.clone()],
        Duration::from_secs(30),
        None,
    )?;

    tracing::info!("got new stream request: {}", req.stream_id);

    let mut client = IngestClient::new(channel);

    let stream = client
        .watch_stream(WatchStreamRequest {
            request_id: req.request_id.clone(),
            stream_id: req.stream_id.clone(),
        })
        .timeout(Duration::from_secs(2))
        .await??
        .into_inner();

    Ok(Job {
        req,
        client,
        stream,
        lock_owner: CancellationToken::new(),
    })
}

struct Job {
    req: TranscoderMessageNewStream,
    client: IngestClient<Channel>,
    stream: tonic::Streaming<WatchStreamResponse>,
    lock_owner: CancellationToken,
}

#[inline(always)]
fn redis_mutex_key(stream_id: &str) -> String {
    format!("transcoder:{}:mutex", stream_id)
}

#[inline(always)]
fn redis_master_playlist_key(stream_id: &str) -> String {
    format!("transcoder:{}:playlist", stream_id)
}

fn set_master_playlist(
    global: Arc<GlobalState>,
    stream_id: &str,
    variants: &[StreamVariant],
    lock: CancellationToken,
) -> impl futures::Future<Output = Result<()>> + Send + 'static {
    let playlist_key = redis_master_playlist_key(stream_id);

    let mut playlist = String::new();

    playlist.push_str("#EXTM3U\n");
    for variant in variants {
        let mut options = vec![
            format!("BANDWIDTH={}", variant.audio_settings.as_ref().map(|a| a.bitrate).unwrap_or_default() + variant.video_settings.as_ref().map(|v| v.bitrate).unwrap_or_default()),
            format!("CODECS=\"{}\"", variant.video_settings.as_ref().map(|v| v.codec.clone()).into_iter().chain(variant.audio_settings.as_ref().map(|a| a.codec.clone()).into_iter()).collect::<Vec<_>>().join(",")),
        ];

        if let Some(video_settings) = &variant.video_settings {
            options.push(format!("RESOLUTION={}x{}", video_settings.width, video_settings.height));
            options.push(format!("FRAME-RATE={}", video_settings.framerate));
        }

        playlist.push_str(&format!(
            "#EXT-X-STREAM-INF:{}\n",
            options.join(",")
        ));

        playlist.push_str(&format!("{}/index.m3u8\n", variant.id))
    }

    async move {
        lock.cancelled().await;

        global.redis.set(&playlist_key, playlist, Some(Expiration::EX(450)), None, false).await?;

        let mut ticker = tokio::time::interval(Duration::from_secs(60));
        loop {
            ticker.tick().await;
            global.redis.expire(&playlist_key, 450).await?;
        }
    }
}

impl Job {
    async fn run(&mut self, global: Arc<GlobalState>, shutdown_token: CancellationToken) {
        tracing::info!("starting transcode job");
        let mut set_lock_fut = pin!(set_lock(
            global.clone(),
            redis_mutex_key(&self.req.stream_id),
            self.req.request_id.clone(),
            self.lock_owner.clone(),
        ));

        let mut update_playlist_fut = pin!(set_master_playlist(
            global.clone(),
            &self.req.stream_id,
            &self.req.variants,
            self.lock_owner.child_token(),
        ));

        // We need to create a unix socket for ffmpeg to connect to.
        let socket_dir = Path::new(&global.config.transcoder.socket_dir).join(&self.req.request_id);
        if let Err(err) = tokio::fs::create_dir_all(&socket_dir).await {
            tracing::error!("failed to create socket dir: {}", err);
            self.report_error("Failed to create socket dir", false)
                .await;
            return;
        }

        let mut futures = FuturesUnordered::new();
        // We need to find the audio only variant.
        let audio_stream = MultiStream::new();

        for v in &self.req.variants {
            let track_setup = if v.name != "audio" {
                let socket = match UnixListener::bind(socket_dir.join(format!("{}.sock", v.id))) {
                    Ok(s) => s,
                    Err(err) => {
                        tracing::error!("failed to bind socket: {}", err);
                        self.report_error("Failed to bind socket", false).await;
                        return;
                    }
                };

                TrackSetup::VideoAudio(socket, audio_stream.subscribe())
            } else {
                TrackSetup::Audio(audio_stream.subscribe())
            };

            futures.push(variant::handle_variant(
                global.clone(),
                self.req.stream_id.clone(),
                v.id.clone(),
                self.req.request_id.clone(),
                track_setup,
            ));
        }

        let custom_variants = self
            .req
            .variants
            .iter()
            .filter(|v| v.name != "source" && v.video_settings.is_some())
            .collect::<Vec<_>>();

        let Some(source_variant) = self
            .req
            .variants
            .iter()
            .find(|v| v.name == "source") else {
                self.report_error("no source variant", true).await;
            tracing::error!("no source variant");
            return;
        };

        let Some(audio_variant) = self
            .req
            .variants
            .iter()
            .find(|v| v.name == "audio") else {
                self.report_error("no audio variant", true).await;
            tracing::error!("no audio variant");
            return;
        };

        let audio_stream_fut = pin!({
            let socket =
                match UnixListener::bind(socket_dir.join(format!("{}.sock", audio_variant.id))) {
                    Ok(s) => s,
                    Err(err) => {
                        self.report_error("Failed to bind socket", false).await;
                        tracing::error!("failed to bind socket: {}", err);
                        return;
                    }
                };

            make_audio_stream(audio_stream, socket)
        });
        let mut audio_stream_fut = SharedFuture::new(audio_stream_fut);

        let filter_graph = custom_variants
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let video = v
                    .video_settings
                    .as_ref()
                    .expect("video settings checked above");

                let previous = if i == 0 {
                    "[0:v]".to_string()
                } else {
                    format!("[{}_out]", i - 1)
                };

                format!(
                    "{}scale={}:{},pad=ceil(iw/2)*2:ceil(ih/2)*2{}",
                    previous,
                    video.width,
                    video.height,
                    if i == custom_variants.len() - 1 {
                        format!("[{}]", v.name)
                    } else {
                        format!(",split=2[{}][{}_out]", v.name, i)
                    }
                )
            })
            .collect::<Vec<_>>()
            .join(";");

        // We need to build a ffmpeg command.
        let Some(audio_settings) = audio_variant
            .audio_settings
            .as_ref() else {
                self.report_error("no audio settings", true).await;
            tracing::error!("no audio settings");
            return;
            };

        const MP4_FLAGS: &str = "+frag_keyframe+empty_moov+default_base_moof";

        #[rustfmt::skip]
        let mut args = vec_of_strings![
            "-v", "error",
            "-i", "-",
            "-probesize", "250M",
            "-analyzeduration", "250M",
            "-map", "0:v",
            "-c:v", "copy",
            "-f", "mp4",
            "-movflags", MP4_FLAGS,
            "-frag_duration", "1",
            format!(
                "unix://{}",
                socket_dir
                    .join(format!("{}.sock", source_variant.id))
                    .display()
            ),
            "-map", "0:a",
            "-c:a", "libopus",
            "-b:a", format!("{}", audio_settings.bitrate),
            "-ac:a", format!("{}", audio_settings.channels),
            "-ar:a", format!("{}", audio_settings.sample_rate),
            "-f", "mp4",
            "-movflags", MP4_FLAGS,
            "-frag_duration", "1",
            format!(
                "unix://{}",
                socket_dir
                    .join(format!("{}.sock", audio_variant.id))
                    .display()
            ),
        ];

        if !filter_graph.is_empty() {
            args.extend(vec_of_strings!["-filter_complex", filter_graph]);
        }

        for v in custom_variants {
            let video = v.video_settings.as_ref().expect("video settings");

            #[rustfmt::skip]
            args.extend(vec_of_strings![
                "-map", format!("[{}]", v.name),
                "-c:v", "libx264",
                "-preset", "medium",
                "-b:v", format!("{}", video.bitrate),
                "-maxrate", format!("{}", video.bitrate),
                "-bufsize", format!("{}", video.bitrate * 2),
                "-profile:v", "high",
                "-level:v", "5.1",
                "-pix_fmt", "yuv420p",
                "-g", format!("{}", video.framerate * 2),
                "-keyint_min", format!("{}", video.framerate * 2),
                "-sc_threshold", "0",
                "-r", format!("{}", video.framerate),
                "-crf", "23",
                "-tune", "zerolatency",
                "-f", "mp4",
                "-movflags", MP4_FLAGS,
                "-frag_duration", "1",
                format!(
                    "unix://{}",
                    socket_dir.join(format!("{}.sock", v.id)).display()
                ),
            ]);
        }

        let mut child = StdCommand::new("ffmpeg");

        child
            .args(&args)
            .stdin(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .process_group(0);

        let mut child = match Command::from(child).spawn() {
            Ok(c) => c,
            Err(err) => {
                tracing::error!("failed to spawn ffmpeg: {}", err);
                self.report_error("failed to spawn ffmpeg", false).await;
                return;
            }
        };

        let mut stdin = child.stdin.take().expect("failed to get stdin");

        let pid = match child.id() {
            Some(pid) => Pid::from_raw(pid as i32),
            None => {
                tracing::error!("failed to get pid");
                self.report_error("failed to get pid", false).await;
                return;
            }
        };

        let child = pin!(child.wait_with_output());
        let mut child = SharedFuture::new(child);

        let mut shutdown_fuse = pin!(shutdown_token.cancelled().fuse());

        while select! {
            _ = &mut shutdown_fuse => {
                self.client.transcoder_event(TranscoderEventRequest {
                    request_id: self.req.request_id.clone(),
                    stream_id: self.req.stream_id.clone(),
                    event: Some(transcoder_event_request::Event::ShuttingDown(true)),
                }).await.is_ok()
            },
            msg = self.stream.next() => self.handle_msg(msg, &mut stdin).await,
            // When FFmpeg exits, we need to exit as well.
            // This is almost always because the stream was closed.
            // So we don't need to report an error, however we check the exit code in the complete_loop function.
            // If the exit code is not 0, we report an error.
            r = &mut child => {
                tracing::info!("ffmpeg exited: {:?}", r);
                false
            },
            // This shutting down usually implies that the stream was closed.
            // So we don't need to report an error.
            _ = &mut audio_stream_fut => {
                tracing::info!("audio stream shutdown while running");
                false
            },
            r = &mut set_lock_fut => {
                if let Err(err) = r {
                    tracing::error!("set lock error: {:#}", err);
                } else {
                    tracing::warn!("set lock done prematurely without error");
                }
                false
            },
            _ = &mut update_playlist_fut => {
                tracing::info!("playlist update shutdown while running");
                false
            },
            // This shutting down usually implies that the stream was closed.
            // So we only report an error if the stream was not closed.
            f = futures.next() => {
                tracing::info!("variant stream shutdown while running");
                if f.unwrap().is_err() {
                    self.report_error("variant stream shutdown while running", true).await;
                }
                false
            },
        } {}

        tracing::info!("shutting down stream");
        drop(stdin);

        if self
            .complete_loop(pid, child, audio_stream_fut, futures.collect::<Vec<_>>())
            .timeout(Duration::from_secs(5))
            .await
            .is_err()
        {
            tracing::error!(
                "failed to shut down stream, forcefully shutting down by stack unwinding"
            );
            self.report_error("failed to shut down stream", false).await;
        };

        if let Err(err) = release_lock(&global, &redis_mutex_key(&self.req.stream_id), &self.req.request_id).await {
            tracing::error!("failed to release lock: {:#}", err);
        };

        tracing::info!("stream shut down");
    }

    async fn complete_loop<V, A>(
        &mut self,
        pid: Pid,
        ffmpeg: impl futures::Future<Output = Arc<Result<Output, io::Error>>> + Unpin,
        audio_stream_fut: impl futures::Future<Output = A> + Unpin,
        variants: impl futures::Future<Output = V> + Unpin,
    ) {
        tracing::info!("waiting for ffmpeg to exit");
        let timeout = ffmpeg.timeout(Duration::from_secs(2)).then(|r| async move {
            if let Ok(r) = r {
                tracing::info!("ffmpeg exited: {:?}", r);

                match r.as_ref() {
                    Ok(r) => !r.status.success(),
                    Err(_) => true,
                }
            } else {
                tracing::warn!("ffmpeg did not exit in time, killing");
                if let Err(err) = signal::kill(pid, signal::Signal::SIGKILL) {
                    tracing::error!("failed to kill ffmpeg: {}", err);
                }

                true
            }
        });

        let (failed, _, _) = join!(timeout, audio_stream_fut, variants);
        if failed {
            self.report_error("ffmpeg exited with non-zero status", false)
                .await;
        }
    }

    async fn handle_msg(
        &mut self,
        msg: Option<Result<WatchStreamResponse, Status>>,
        stdin: &mut ChildStdin,
    ) -> bool {
        let msg = match msg {
            Some(Ok(msg)) => msg.data,
            _ => {
                // We should have gotten a shutting down event
                // TODO: report this to API server
                tracing::error!("unexpected stream closed");
                return false;
            }
        };

        let Some(msg) = msg else {
            tracing::error!("recieved empty response");
            return false;
        };

        match msg {
            watch_stream_response::Data::InitSegment(data) => {
                if stdin.write_all(&data).await.is_err() {
                    // This is almost always because ffmpeg crashed
                    // We report an error when we check the exit code
                    return false;
                }
            }
            watch_stream_response::Data::MediaSegment(ms) => {
                if stdin.write_all(&ms.data).await.is_err() {
                    // This is almost always because ffmpeg crashed
                    // We report an error when we check the exit code
                    return false;
                }
            }
            watch_stream_response::Data::ShuttingDown(stream) => {
                tracing::info!(stream = stream, "shutting down");
                return false;
            }
        }

        true
    }

    async fn report_error(&mut self, err: impl ToString + Send + Sync, fatal: bool) {
        if let Err(err) = self
            .client
            .transcoder_event(TranscoderEventRequest {
                request_id: self.req.request_id.clone(),
                stream_id: self.req.stream_id.clone(),
                event: Some(transcoder_event_request::Event::Error(
                    transcoder_event_request::Error {
                        message: err.to_string(),
                        fatal,
                    },
                )),
            })
            .await
        {
            tracing::error!("failed to report error: {}", err);
        }
    }
}
