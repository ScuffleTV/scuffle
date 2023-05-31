use std::collections::HashMap;
use std::io;
use std::process::Output;
use std::{
    os::unix::process::CommandExt, path::Path, pin::pin, process::Command as StdCommand, sync::Arc,
    time::Duration,
};

use anyhow::{anyhow, Result};
use async_stream::stream;
use common::prelude::*;
use common::vec_of_strings;
use fred::types::Expiration;
use futures::{stream::FuturesUnordered, FutureExt, StreamExt};
use futures_util::Stream;
use lapin::message::Delivery;
use lapin::options::BasicAckOptions;
use mp4::codec::{AudioCodec, VideoCodec};
use nix::sys::signal;
use nix::unistd::Pid;
use prost::Message as _;
use tokio::sync::mpsc;
use tokio::{
    io::AsyncWriteExt,
    net::UnixListener,
    process::{ChildStdin, Command},
    select,
};
use tokio_util::sync::CancellationToken;
use tonic::{transport::Channel, Status};

use crate::pb::scuffle::types::{stream_variants, StreamVariants};
use crate::transcoder::job::utils::{release_lock, set_lock, SharedFuture};
use crate::{
    global::GlobalState,
    pb::scuffle::{
        events::{transcoder_message, TranscoderMessage, TranscoderMessageNewStream},
        video::{
            ingest_client::IngestClient, transcoder_event_request, watch_stream_response,
            TranscoderEventRequest, WatchStreamRequest, WatchStreamResponse,
        },
    },
};
use fred::interfaces::KeysInterface;

mod track_parser;
mod utils;
pub(crate) mod variant;

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
    state: &StreamVariants,
    lock: CancellationToken,
) -> impl futures::Future<Output = Result<()>> + Send + 'static {
    let playlist_key = redis_master_playlist_key(stream_id);

    let mut playlist = String::new();

    playlist.push_str("#EXTM3U\n");

    let mut state_map = HashMap::new();

    for transcode_state in state.transcode_states.iter() {
        playlist.push_str(format!("#EXT-X-MEDIA:TYPE={},GROUP-ID=\"{}\",NAME=\"{}\",AUTOSELECT=YES,DEFAULT=YES,URI=\"{}/index.m3u8\"\n", match transcode_state.settings.as_ref().unwrap() {
            stream_variants::transcode_state::Settings::Video(_) => {
                "VIDEO"
            },
            stream_variants::transcode_state::Settings::Audio(_) => {
                "AUDIO"
            },
        }, transcode_state.id, transcode_state.id, transcode_state.id).as_str());

        state_map.insert(transcode_state.id.as_str(), transcode_state);
    }

    for stream_variant in state.stream_variants.iter() {
        let video_transcode_state = stream_variant.transcode_state_ids.iter().find_map(|id| {
            let t = state_map.get(id.as_str()).unwrap();
            if matches!(
                t.settings,
                Some(stream_variants::transcode_state::Settings::Video(_))
            ) {
                Some(t)
            } else {
                None
            }
        });

        let audio_transcode_state = stream_variant.transcode_state_ids.iter().find_map(|id| {
            let t = state_map.get(id.as_str()).unwrap();
            if matches!(
                t.settings,
                Some(stream_variants::transcode_state::Settings::Audio(_))
            ) {
                Some(t)
            } else {
                None
            }
        });

        let bandwidth = video_transcode_state.map(|t| t.bitrate).unwrap_or(0)
            + audio_transcode_state.map(|t| t.bitrate).unwrap_or(0);
        let codecs = video_transcode_state
            .iter()
            .chain(audio_transcode_state.iter())
            .map(|t| t.codec.as_str())
            .collect::<Vec<_>>()
            .join(",");

        let mut tags = vec![
            format!("GROUP=\"{}\"", stream_variant.group),
            format!("NAME=\"{}\"", stream_variant.name),
            format!("BANDWIDTH={}", bandwidth),
            format!("CODECS=\"{}\"", codecs),
        ];

        if let Some(video) = video_transcode_state {
            let settings = match video.settings.as_ref() {
                Some(stream_variants::transcode_state::Settings::Video(settings)) => settings,
                _ => unreachable!(),
            };

            tags.push(format!("RESOLUTION={}x{}", settings.width, settings.height));
            tags.push(format!("FRAME-RATE={}", settings.framerate));
            tags.push(format!("VIDEO=\"{}\"", video.id));
        }

        if let Some(audio) = audio_transcode_state {
            tags.push(format!("AUDIO=\"{}\"", audio.id));
        }

        playlist.push_str(
            format!(
                "#EXT-X-STREAM-INF:{}\n{}/index.m3u8\n",
                tags.join(","),
                video_transcode_state.or(audio_transcode_state).unwrap().id
            )
            .as_str(),
        );
    }

    async move {
        lock.cancelled().await;

        global
            .redis
            .set(
                &playlist_key,
                playlist,
                Some(Expiration::EX(450)),
                None,
                false,
            )
            .await?;

        let mut ticker = tokio::time::interval(Duration::from_secs(60));
        loop {
            ticker.tick().await;
            global.redis.expire(&playlist_key, 450).await?;
        }
    }
}

fn report_to_ingest(
    global: Arc<GlobalState>,
    mut client: IngestClient<Channel>,
    mut channel: mpsc::Receiver<TranscoderEventRequest>,
) -> impl Stream<Item = Result<()>> + Send + 'static {
    stream! {
        loop {
            select! {
                msg = channel.recv() => {
                    match msg {
                        Some(msg) => {
                            match client.transcoder_event(msg).timeout(Duration::from_secs(5)).await {
                                Ok(Ok(_)) => {},
                                Ok(Err(e)) => {
                                    yield Err(e.into());
                                }
                                Err(e) => {
                                    yield Err(e.into());
                                }
                            }
                        },
                        None => {
                            break;
                        }
                    }
                },
                _ = global.ctx.done() => {
                    break;
                }
            }
        }
    }
}

impl Job {
    fn variants(&self) -> &StreamVariants {
        self.req.variants.as_ref().unwrap()
    }

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
            self.req.variants.as_ref().unwrap(),
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

        let variants = self.variants();

        let (ready_tx, mut ready_recv) = mpsc::channel(16);

        for transcode_state in variants.transcode_states.iter() {
            let sock_path = socket_dir.join(format!("{}.sock", transcode_state.id));
            let socket = match UnixListener::bind(&sock_path) {
                Ok(s) => s,
                Err(err) => {
                    tracing::error!("failed to bind socket: {}", err);
                    self.report_error("Failed to bind socket", false).await;
                    return;
                }
            };

            // Change user and group of the socket.
            if let Err(err) = nix::unistd::chown(
                sock_path.as_os_str(),
                Some(nix::unistd::Uid::from_raw(global.config.transcoder.uid)),
                Some(nix::unistd::Gid::from_raw(global.config.transcoder.gid)),
            ) {
                tracing::error!("failed to chown socket: {}", err);
                self.report_error("Failed to chown socket", false).await;
                return;
            }

            futures.push(variant::handle_variant(
                global.clone(),
                ready_tx.clone(),
                self.req.stream_id.clone(),
                transcode_state.id.clone(),
                self.req.request_id.clone(),
                socket,
            ));
        }

        let filter_graph_items = self
            .variants()
            .transcode_states
            .iter()
            .filter(|v| {
                !v.copy
                    && matches!(
                        v.settings,
                        Some(stream_variants::transcode_state::Settings::Video(_))
                    )
            })
            .collect::<Vec<_>>();

        let filter_graph = filter_graph_items
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let settings = match v.settings.as_ref().unwrap() {
                    stream_variants::transcode_state::Settings::Video(v) => v,
                    _ => unreachable!(),
                };

                let previous = if i == 0 {
                    "[0:v]".to_string()
                } else {
                    format!("[{}_out]", i - 1)
                };

                format!(
                    "{}scale={}:{},pad=ceil(iw/2)*2:ceil(ih/2)*2{}",
                    previous,
                    settings.width,
                    settings.height,
                    if i == filter_graph_items.len() - 1 {
                        format!("[{}]", v.id)
                    } else {
                        format!(",split=2[{}][{}_out]", v.id, i)
                    }
                )
            })
            .collect::<Vec<_>>()
            .join(";");

        const MP4_FLAGS: &str = "+frag_keyframe+empty_moov+default_base_moof";

        #[rustfmt::skip]
        let mut args = vec_of_strings![
            "-v", "error",
            "-i", "-",
            "-probesize", "250M",
            "-analyzeduration", "250M",
        ];

        if !filter_graph.is_empty() {
            args.extend(vec_of_strings!["-filter_complex", filter_graph]);
        }

        for state in variants.transcode_states.iter() {
            match state.settings {
                Some(stream_variants::transcode_state::Settings::Video(ref video)) => {
                    if state.copy {
                        #[rustfmt::skip]
                        args.extend(vec_of_strings![
                            "-map", "0:v",
                            "-c:v", "copy",
                        ]);
                    } else {
                        let codec: VideoCodec = match state.codec.parse() {
                            Ok(c) => c,
                            Err(err) => {
                                tracing::error!("invalid video codec: {}", err);
                                self.report_error("Invalid video codec", false).await;
                                return;
                            }
                        };

                        match codec {
                            VideoCodec::Avc { profile, level, .. } => {
                                #[rustfmt::skip]
                                args.extend(vec_of_strings![
                                    "-map", format!("[{}]", state.id),
                                    "-c:v", "libx264",
                                    "-preset", "medium",
                                    "-b:v", format!("{}", state.bitrate),
                                    "-maxrate", format!("{}", state.bitrate),
                                    "-bufsize", format!("{}", state.bitrate * 2),
                                    "-profile:v", match profile {
                                        66 => "baseline",
                                        77 => "main",
                                        100 => "high",
                                        _ => {
                                            tracing::error!("invalid avc profile: {}", profile);
                                            self.report_error("Invalid avc profile", false).await;
                                            return;
                                        },
                                    },
                                    "-level:v", match level {
                                        30 => "3.0",
                                        31 => "3.1",
                                        32 => "3.2",
                                        40 => "4.0",
                                        41 => "4.1",
                                        42 => "4.2",
                                        50 => "5.0",
                                        51 => "5.1",
                                        52 => "5.2",
                                        60 => "6.0",
                                        61 => "6.1",
                                        62 => "6.2",
                                        _ => {
                                            tracing::error!("invalid avc level: {}", level);
                                            self.report_error("Invalid avc level", false).await;
                                            return;
                                        },
                                    },
                                    "-pix_fmt", "yuv420p",
                                    "-g", format!("{}", video.framerate * 2),
                                    "-keyint_min", format!("{}", video.framerate * 2),
                                    "-sc_threshold", "0",
                                    "-r", format!("{}", video.framerate),
                                    "-crf", "23",
                                    "-tune", "zerolatency",
                                ]);
                            }
                            VideoCodec::Av1 { .. } => {
                                tracing::error!("av1 is not supported");
                                self.report_error("AV1 is not supported", false).await;
                                return;
                            }
                            VideoCodec::Hevc { .. } => {
                                tracing::error!("hevc is not supported");
                                self.report_error("HEVC is not supported", false).await;
                                return;
                            }
                        }
                    }
                }
                Some(stream_variants::transcode_state::Settings::Audio(ref audio)) => {
                    if state.copy {
                        tracing::error!("audio copy is not supported");
                        self.report_error("Audio copy is not supported", false)
                            .await;
                        return;
                    } else {
                        let codec: AudioCodec = match state.codec.parse() {
                            Ok(c) => c,
                            Err(err) => {
                                tracing::error!("invalid audio codec: {}", err);
                                self.report_error("Invalid audio codec", false).await;
                                return;
                            }
                        };

                        match codec {
                            AudioCodec::Aac { object_type } => {
                                args.extend(vec_of_strings![
                                    "-map",
                                    "0:a",
                                    "-c:a",
                                    "aac",
                                    "-b:a",
                                    format!("{}", state.bitrate),
                                    "-ar",
                                    format!("{}", audio.sample_rate),
                                    "-ac",
                                    format!("{}", audio.channels),
                                    "-profile:a",
                                    match object_type {
                                        aac::AudioObjectType::AacLowComplexity => {
                                            "aac_low"
                                        }
                                        aac::AudioObjectType::AacMain => {
                                            "aac_main"
                                        }
                                        aac::AudioObjectType::Unknown(profile) => {
                                            tracing::error!("invalid aac profile: {}", profile);
                                            self.report_error("Invalid aac profile", false).await;
                                            return;
                                        }
                                    },
                                ]);
                            }
                            AudioCodec::Opus => {
                                args.extend(vec_of_strings![
                                    "-map",
                                    "0:a",
                                    "-c:a",
                                    "libopus",
                                    "-b:a",
                                    format!("{}", state.bitrate),
                                    "-ar",
                                    format!("{}", audio.sample_rate),
                                    "-ac",
                                    format!("{}", audio.channels),
                                ]);
                            }
                        }
                    }
                }
                None => {
                    tracing::error!("no settings for variant {}", state.id);
                    self.report_error("No settings for variant", true).await;
                    return;
                }
            }

            // Common args regardless of copy or transcode mode
            #[rustfmt::skip]
            args.extend(vec_of_strings![
                "-f", "mp4",
                "-movflags", MP4_FLAGS,
                "-frag_duration", "1",
                format!(
                    "unix://{}",
                    socket_dir.join(format!("{}.sock", state.id)).display()
                ),
            ]);
        }

        let mut child = StdCommand::new("ffmpeg");

        child
            .args(&args)
            .stdin(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .process_group(0)
            .uid(global.config.transcoder.uid)
            .gid(global.config.transcoder.gid)
            .env_clear()
            .env("PATH", std::env::var("PATH").unwrap_or_default());

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

        let mut ready_count = 0;

        let (report, rx) = mpsc::channel(10);
        let mut report_fut = pin!(report_to_ingest(global.clone(), self.client.clone(), rx));

        while select! {
            r = report_fut.next() => {
                tracing::info!("reporting to ingest failed: {:#?}", r);
                false
            },
            _ = &mut shutdown_fuse => {
                report.try_send(TranscoderEventRequest {
                    request_id: self.req.request_id.clone(),
                    stream_id: self.req.stream_id.clone(),
                    event: Some(transcoder_event_request::Event::ShuttingDown(true)),
                }).is_ok()
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
            _ = ready_recv.recv() => {
                ready_count += 1;
                if ready_count == self.variants().transcode_states.len() {
                    tracing::info!("all variants ready");
                    report.try_send(TranscoderEventRequest {
                        request_id: self.req.request_id.clone(),
                        stream_id: self.req.stream_id.clone(),
                        event: Some(transcoder_event_request::Event::Started(true)),
                    }).is_ok()
                } else {
                    true
                }
            }
        } {}

        tracing::debug!("shutting down");
        drop(stdin);

        select! {
            r = self.complete_loop(pid, child, futures.collect::<Vec<_>>()).timeout(Duration::from_secs(5)) => {
                if let Err(err) = r {
                    tracing::error!("failed to complete loop: {:#}", err);
                    self.report_error("failed to complete loop", false).await;
                }
            },
            r = set_lock_fut => {
                if let Err(err) = r {
                    tracing::error!("set lock error: {:#}", err);
                } else {
                    tracing::warn!("set lock done prematurely without error");
                }
            },
        }

        drop(report);

        // Finish all the report futures
        while report_fut.next().await.is_some() {}

        tracing::info!("waiting for playlist update to exit");

        if let Err(err) = release_lock(
            &global,
            &redis_mutex_key(&self.req.stream_id),
            &self.req.request_id,
        )
        .timeout(Duration::from_secs(2))
        .await
        {
            tracing::error!("failed to release lock: {:#}", err);
        };

        tracing::info!("stream shut down");
    }

    async fn complete_loop<V>(
        &mut self,
        pid: Pid,
        mut ffmpeg: impl futures::Future<Output = Arc<Result<Output, io::Error>>> + Unpin,
        mut variants: impl futures::Future<Output = V> + Unpin,
    ) {
        tracing::info!("waiting for ffmpeg to exit");

        let pid = pid.as_raw();

        let mut timeout = pin!((&mut ffmpeg)
            .timeout(Duration::from_millis(400))
            .then(|r| async {
                if let Ok(r) = r {
                    tracing::info!("ffmpeg exited: {:?}", r);

                    Some(match r.as_ref() {
                        Ok(r) => !r.status.success(),
                        Err(_) => true,
                    })
                } else {
                    signal::kill(Pid::from_raw(pid), signal::Signal::SIGTERM).ok();
                    tracing::debug!("ffmpeg did not exit in time, sending SIGTERM");

                    None
                }
            }));

        let mut variants_done = false;
        let r = select! {
            r = &mut timeout => r,
            _ = &mut variants => {
                tracing::info!("variants exited");
                variants_done = true;
                timeout.await
            },
        };

        let failed = if let Some(r) = r {
            Some(r)
        } else {
            let timeout = ffmpeg.timeout(Duration::from_secs(2)).then(|r| async {
                if let Ok(r) = r {
                    tracing::info!("ffmpeg exited: {:?}", r);

                    Some(match r.as_ref() {
                        Ok(r) => !r.status.success(),
                        Err(_) => true,
                    })
                } else {
                    None
                }
            });

            if variants_done {
                timeout.await
            } else {
                variants_done = true;
                tokio::join!(timeout, &mut variants).0
            }
        };

        let failed = failed.unwrap_or_else(|| {
            tracing::error!("ffmpeg did not exit in time, sending SIGKILL");
            signal::kill(Pid::from_raw(pid), signal::Signal::SIGKILL).ok();
            true
        });

        if !variants_done {
            tracing::info!("waiting for variants to exit");
            variants.await;
        }

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
        tracing::debug!("recieved message");
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
            .timeout(Duration::from_secs(2))
            .await
        {
            tracing::error!("failed to report error: {}", err);
        }
    }
}
