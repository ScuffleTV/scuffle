use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::{pin, Pin};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use async_nats::jetstream::Message;
use bytes::Bytes;
use common::prelude::FutureTimeout;
use futures::{FutureExt, StreamExt};
use futures_util::{Future, Stream, TryFutureExt};
use pb::ext::UlidExt;
use pb::scuffle::video::internal::events::TranscoderRequestTask;
use pb::scuffle::video::internal::ingest_client::IngestClient;
use pb::scuffle::video::internal::{
	ingest_watch_request, ingest_watch_response, live_rendition_manifest, IngestWatchRequest, IngestWatchResponse,
	LiveManifest, LiveRenditionManifest,
};
use pb::scuffle::video::v1::events_fetch_request::Target;
use pb::scuffle::video::v1::types::{event, VideoConfig};
use prost::Message as _;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use tokio::process::{Child, ChildStdin};
use tokio::sync::mpsc;
use tokio::time::Instant;
use tokio::{select, try_join};
use tokio_util::sync::CancellationToken;
use tonic::transport::Channel;
use tonic::Status;
use ulid::Ulid;
use uuid::Uuid;
use video_common::database::{Rendition, RoomStatus};

use self::renditions::screenshot_size;
use self::track_parser::TrackOut;
use self::utils::{delete_media_generator, upload_metadata_generator, Recording, TaskDomain, TaskError, TrackState};
use crate::config::TranscoderConfig;
use crate::global::TranscoderGlobal;
use crate::transcoder::job::track_parser::track_parser;
use crate::transcoder::job::utils::{
	bind_socket, perform_sql_operations, spawn_ffmpeg, spawn_ffmpeg_screenshot, unix_stream, upload_media_generator,
	MultiTasker,
};

mod renditions;
mod track_parser;
mod utils;

pub async fn handle_message<G: TranscoderGlobal>(global: Arc<G>, msg: Message, shutdown_token: CancellationToken) {
	let mut job = match Job::new(&global, &msg).await {
		Ok(job) => job,
		Err(err) => {
			msg.ack_with(async_nats::jetstream::AckKind::Nak(Some(Duration::from_secs(15))))
				.await
				.ok();
			tracing::error!(error = %err, "failed to handle message");
			return;
		}
	};

	if let Err(err) = msg.double_ack().await {
		tracing::error!(error = %err, "failed to ACK message");
		return;
	};

	let mut streams = futures::stream::select_all(
		job.tracks
			.drain(..)
			.map(|(t, rendition)| Box::pin(track_parser(unix_stream(t, 1024 * 1024))).map(move |r| (r, rendition))),
	);

	if let Err(err) = job.run(&global, shutdown_token, &mut streams).await {
		video_common::events::emit(
			global.nats(),
			job.organization_id,
			Target::Room,
			event::Event::Room(event::Room {
				room_id: Some(job.room_id.into()),
				event: Some(event::room::Event::Failed(event::room::Failed {
					connection_id: Some(job.connection_id.into()),
					error: err.to_string(),
				})),
			}),
		)
		.await;

		tracing::error!(error = %err, "failed to run transcoder");
	}

	if let Err(err) = job.handle_shutdown(&global, &mut streams).await {
		tracing::error!(error = %err, "failed to shutdown transcoder");
	}

	tracing::info!("stream finished");
}

type TaskFuture<T> = Pin<Box<dyn Future<Output = anyhow::Result<T>> + Send>>;

struct Ffmpeg {
	process: Child,
	stdin: Option<ChildStdin>,
}

struct Job<G: TranscoderGlobal> {
	organization_id: Ulid,
	room_id: Ulid,
	connection_id: Ulid,
	_socket_dir: CleanupPath,

	video_input: VideoConfig,
	recording: Option<Recording>,

	ingest_ready: bool,
	transcoder_ready: bool,
	init_segment: Option<Bytes>,

	track_state: HashMap<Rendition, TrackState>,
	manifests: HashMap<Rendition, LiveRenditionManifest>,

	tasker: MultiTasker<G>,

	screenshot_task: Option<TaskFuture<(Bytes, f64)>>,

	ffmpeg: Ffmpeg,

	tracks: Vec<(UnixListener, Rendition)>,

	_client: IngestClient<Channel>,

	shutdown: Option<ingest_watch_response::Shutdown>,

	first_init_put: bool,

	screenshot_idx: u32,

	last_screenshot: Instant,

	send: mpsc::Sender<IngestWatchRequest>,
	recv: tonic::Streaming<IngestWatchResponse>,
}

struct CleanupPath(PathBuf);

impl Drop for CleanupPath {
	fn drop(&mut self) {
		let path = self.0.clone();
		tokio::spawn(async move {
			if let Err(err) = tokio::fs::remove_dir_all(path).await {
				tracing::error!(error = %err, "failed to cleanup socket dir");
			}
		});
	}
}

impl<G: TranscoderGlobal> Job<G> {
	async fn new(global: &Arc<G>, msg: &Message) -> Result<Self> {
		let message = TranscoderRequestTask::decode(msg.payload.clone())?;

		let organization_id = message.organization_id.into_ulid();
		let room_id = message.room_id.into_ulid();
		let connection_id = message.connection_id.into_ulid();

		let result = perform_sql_operations(global, organization_id, room_id, connection_id).await?;

		tracing::info!(
			%organization_id,
			%room_id,
			%connection_id,
			transcoding_config_id = %result.transcoding_config.id.into_ulid(),
			recording_id = %result.recording.as_ref().map(|r| r.id().to_string()).unwrap_or_default(),
			"got new stream request",
		);

		let config = global.config::<TranscoderConfig>();

		// We need to create a unix socket for ffmpeg to connect to.
		let socket_dir = CleanupPath(Path::new(&config.socket_dir).join(message.request_id.into_ulid().to_string()));
		if let Err(err) = tokio::fs::create_dir_all(&socket_dir.0).await {
			anyhow::bail!("failed to create socket dir: {}", err)
		}

		let tracks = result
			.video_output
			.iter()
			.map(|output| output.rendition())
			.chain(result.audio_output.iter().map(|output| output.rendition()))
			.map(Rendition::from)
			.map(|rendition| {
				let sock_path = socket_dir.0.join(format!("{rendition}.sock"));
				let socket = bind_socket(&sock_path, config.ffmpeg_uid, config.ffmpeg_gid)?;

				Ok((socket, rendition))
			})
			.collect::<Result<Vec<_>>>()?;

		let mut ffmpeg = spawn_ffmpeg(
			config.ffmpeg_gid,
			config.ffmpeg_uid,
			&socket_dir.0,
			&result.video_output,
			&result.audio_output,
		)?;

		tracing::debug!(endpoint = %message.grpc_endpoint, "trying to connect to ingest");

		let tls = global.ingest_tls();

		let channel = common::grpc::make_channel(vec![message.grpc_endpoint], Duration::from_secs(30), tls)?;

		let mut client = IngestClient::new(channel);

		let (send, rx) = mpsc::channel(16);

		send.try_send(IngestWatchRequest {
			message: Some(ingest_watch_request::Message::Open(ingest_watch_request::Open {
				request_id: message.request_id,
			})),
		})
		.ok();

		let recv = client
			.watch(tokio_stream::wrappers::ReceiverStream::new(rx))
			.timeout(Duration::from_secs(2))
			.await
			.context("failed to connect to ingest")??
			.into_inner();

		let mut tasker = MultiTasker::new();

		tasker.add_domain(TaskDomain::Generic);
		tasker.add_domain(TaskDomain::Thumbnail);
		for rendition in tracks.iter().map(|(_, r)| *r) {
			tasker.add_domain(TaskDomain::Normal(rendition));
		}
		if let Some(recording) = &result.recording {
			for rendition in recording.renditions() {
				tasker.add_domain(TaskDomain::Recording(*rendition));
			}
		}

		Ok(Self {
			organization_id,
			room_id,
			connection_id,
			_socket_dir: socket_dir,
			recording: result.recording,
			_client: client,
			ffmpeg: Ffmpeg {
				stdin: ffmpeg.stdin.take(),
				process: ffmpeg,
			},
			init_segment: None,
			shutdown: None,
			tasker,
			screenshot_task: None,
			last_screenshot: Instant::now(),
			screenshot_idx: 0,
			video_input: result.video_input,
			manifests: tracks
				.iter()
				.map(|(_, rendition)| (*rendition, LiveRenditionManifest::default()))
				.collect(),
			ingest_ready: false,
			transcoder_ready: false,
			track_state: tracks
				.iter()
				.map(|(_, rendition)| (*rendition, TrackState::default()))
				.collect(),
			send,
			recv,
			tracks,
			first_init_put: true,
		})
	}

	async fn run(
		&mut self,
		global: &Arc<G>,
		shutdown_token: CancellationToken,
		mut streams: impl Stream<Item = (io::Result<TrackOut>, Rendition)> + Unpin,
	) -> Result<()> {
		tracing::info!("starting transcode job");

		let mut shutdown_fuse = pin!(shutdown_token.cancelled().fuse());

		let mut upload_init_timer = tokio::time::interval(Duration::from_secs(15));

		loop {
			select! {
				_ = &mut shutdown_fuse => {
					self.send.try_send(IngestWatchRequest {
						message: Some(ingest_watch_request::Message::Shutdown(
							ingest_watch_request::Shutdown::Request as i32,
						))
					})?;
				},
				msg = self.recv.next() => {
					let mut items = Vec::new();

					{
						let mut msg_handle_fut = self.handle_msg(global, msg);
						let mut msg_handle_fut = pin!(msg_handle_fut);
						loop {
							select! {
								r = &mut msg_handle_fut => {
									r?;
									break;
								},
								Some(item) = streams.next() => {
									tracing::debug!(count = %items.len() + 1, "got item while handling message");
									items.push(item);
								}
							}
						}
					}

					for item in items {
						let (result, rendition) = item;
						self.handle_track(global, rendition, result)?;
					}
				},
				r = self.ffmpeg.process.wait() => {
					r?;
					break;
				},
				Some(result) = self.tasker.next_task(global.clone()) => {
					match result {
						Err((task, err)) => {
							if task.retry_count() < 5 {
								tracing::error!(error = %err, retry = task.retry_count(), id = task.id(), domain = ?task.domain(), "failed to upload media");
								self.tasker.requeue(task);
							} else {
								anyhow::bail!("failed to upload media after 5 retries: {}", err);
							}
						}
						Ok(task) => {
							tracing::debug!(key = %task.id(), domain = ?task.domain(), "completed task");
						}
					}
				},
				Some(screenshot) = async {
					if let Some(task) = self.screenshot_task.as_mut() {
						let r = task.await;
						self.screenshot_task = None;
						Some(r)
					} else {
						tracing::trace!("no screenshot to process");
						None
					}
				} => {
					let (screenshot, start_time) = screenshot?;

					self.screenshot_idx += 1;

					let id = Ulid::new();

					let key = video_common::keys::screenshot(
						self.organization_id,
						self.room_id,
						self.connection_id,
						self.screenshot_idx,
					);

					self.tasker.submit(
						TaskDomain::Generic,
						format!("screenshot_{idx}", idx = self.screenshot_idx),
						upload_media_generator(key, screenshot.clone()),
					);

					if let Some(recording) = &mut self.recording {
						self.tasker.submit_task(recording.upload_thumbnail(id, self.screenshot_idx, start_time, screenshot));
					}

					self.update_manifest();
					self.ready();
				}
				r = streams.next() => {
					let Some((result, rendition)) = r else {
						break;
					};

					self.handle_track(global, rendition, result)?;
				},
				_ = upload_init_timer.tick() => {
					self.put_init_segments()?;
					self.update_manifest();
				}
			}
		}

		Ok(())
	}

	fn ready(&mut self) {
		if self.transcoder_ready {
			return;
		}

		self.transcoder_ready = true;

		let organization_id = self.organization_id;
		let connection_id = self.connection_id;
		let room_id = self.room_id;

		self.tasker
			.submit_with_abort(TaskDomain::Generic, "room_ready", move |global| {
				let global = global.clone();
				Box::pin(async move {
					let resp = sqlx::query(
						r#"
						UPDATE rooms
						SET
							updated_at = NOW(),
							status = $1
						WHERE
							organization_id = $2 AND
							id = $3 AND
							active_ingest_connection_id = $4
						"#,
					)
					.bind(RoomStatus::Ready)
					.bind(Uuid::from(organization_id))
					.bind(Uuid::from(room_id))
					.bind(Uuid::from(connection_id))
					.execute(global.db().as_ref())
					.await
					.map_err(|e| TaskError::Custom(e.into()))?;

					if resp.rows_affected() != 1 {
						return Err(TaskError::Custom(anyhow::anyhow!("failed to update room status")));
					}

					video_common::events::emit(
						global.nats(),
						organization_id,
						Target::Room,
						event::Event::Room(event::Room {
							room_id: Some(room_id.into()),
							event: Some(event::room::Event::Ready(event::room::Ready {
								connection_id: Some(connection_id.into()),
							})),
						}),
					)
					.await;

					Ok(())
				})
			});
	}

	async fn handle_msg(&mut self, global: &Arc<G>, msg: Option<Result<IngestWatchResponse, Status>>) -> Result<()> {
		tracing::trace!("recieved message");

		let Some(Ok(msg)) = msg else {
			if self.shutdown.is_none() {
				anyhow::bail!("ingest stream closed")
			}

			return Ok(());
		};

		let msg = msg.message.ok_or_else(|| anyhow::anyhow!("ingest sent bad message"))?;

		match msg {
			ingest_watch_response::Message::Media(media) => {
				if let Some(stdin) = &mut self.ffmpeg.stdin {
					stdin
						.write_all(&media.data)
						.timeout(Duration::from_secs(1))
						.await
						.context("ffmped is blocked")?
						.context("ffmpeg stdin closed")?;
				} else {
					anyhow::bail!("ffmpeg stdin was not open");
				}

				match media.r#type() {
					ingest_watch_response::media::Type::Init => {
						self.init_segment = Some(media.data.clone());
					}
					ingest_watch_response::media::Type::Video => {
						if media.keyframe
							&& self.last_screenshot.elapsed() > Duration::from_secs(5)
							&& self.screenshot_task.is_none()
						{
							self.take_screenshot(global, &media.data, media.timestamp as f64 / media.timescale as f64)?;
						}
					}
					ingest_watch_response::media::Type::Audio => {}
				}
			}
			ingest_watch_response::Message::Shutdown(s) => {
				self.shutdown = Some(
					ingest_watch_response::Shutdown::try_from(s).unwrap_or(ingest_watch_response::Shutdown::Transcoder),
				);
				self.ffmpeg.stdin.take();
			}
			ingest_watch_response::Message::Ready(_) => {
				self.ingest_ready = true;
				self.fetch_manifests(global).await?;
				self.put_init_segments()?;
				for rendition in self.track_state.keys().cloned().collect::<Vec<_>>() {
					self.handle_sample(global, rendition)?;
				}
				tracing::info!("ingest reported ready");
			}
		}

		Ok(())
	}

	fn take_screenshot(&mut self, global: &Arc<G>, data: &Bytes, start_time: f64) -> Result<()> {
		if !self.ingest_ready {
			return Ok(());
		}

		if let Some(init_segment) = self.init_segment.clone() {
			let (width, height) = screenshot_size(&self.video_input);

			let config = global.config::<TranscoderConfig>();

			let mut child = spawn_ffmpeg_screenshot(config.ffmpeg_gid, config.ffmpeg_uid, width, height)?;

			self.last_screenshot = Instant::now();

			tracing::debug!("taking screenshot");

			let data = data.clone();

			self.screenshot_task = Some(Box::pin(async move {
				let stdin = child.stdin.as_mut().unwrap();
				stdin.write_all(&init_segment).await?;
				stdin.write_all(&data).await?;
				stdin.flush().await?;

				let start = Instant::now();
				let output = child.wait_with_output().await?;
				if !output.status.success() {
					tracing::error!("screenshot stdout: {}", String::from_utf8_lossy(&output.stderr));
				}

				let duration = format!("{:.5}ms", start.elapsed().as_secs_f64() * 1000.0);

				tracing::debug!(duration, "screenshot captured");

				Ok((Bytes::from(output.stdout), start_time))
			}));
		}

		Ok(())
	}

	fn handle_track(&mut self, global: &Arc<G>, rendition: Rendition, result: io::Result<TrackOut>) -> Result<()> {
		match result? {
			TrackOut::Moov(moov) => {
				self.track_state.get_mut(&rendition).unwrap().set_moov(moov);
				self.put_init_segments()?;
			}
			TrackOut::Samples(samples) => {
				self.track_state.get_mut(&rendition).unwrap().append_samples(samples);
				self.handle_sample(global, rendition)?;
			}
		}

		Ok(())
	}

	fn put_init_segments(&mut self) -> Result<()> {
		if !self.ingest_ready {
			return Ok(());
		}

		if self.track_state.iter().any(|(_, state)| state.init_segment().is_none()) {
			return Ok(());
		}

		self.track_state.iter().for_each(|(rendition, state)| {
			let key = video_common::keys::init(self.organization_id, self.room_id, self.connection_id, *rendition);

			let data = state.init_segment().unwrap().clone();
			self.tasker
				.submit_with_abort(TaskDomain::Normal(*rendition), "init", upload_media_generator(key, data));
		});

		if self.first_init_put {
			self.first_init_put = false;

			if let Some(recording) = &mut self.recording {
				self.track_state.iter().for_each(|(rendition, state)| {
					self.tasker
						.submit_task_with_abort(recording.upload_init(*rendition, state.init_segment().unwrap().clone()));
				});
			}
		}

		Ok(())
	}

	fn handle_sample(&mut self, global: &Arc<G>, rendition: Rendition) -> Result<()> {
		if !self.ingest_ready || self.first_init_put {
			return Ok(());
		}

		let track_state = self.track_state.get_mut(&rendition).unwrap();

		let config = global.config::<TranscoderConfig>();

		let additions = track_state.split_samples(
			config.target_part_duration.as_secs_f64(),
			config.max_part_duration.as_secs_f64(),
			config.min_segment_duration.as_secs_f64(),
		);

		for (segment_idx, parts) in additions {
			for part_idx in parts {
				let key =
					video_common::keys::part(self.organization_id, self.room_id, self.connection_id, rendition, part_idx);

				let segment = track_state.segment(segment_idx).unwrap();

				let part = segment.part(part_idx).unwrap();

				if let Some(recording) = &mut self.recording {
					if let Some(task) = recording.upload_part(
						rendition,
						segment.id,
						segment.idx,
						part.data.clone(),
						segment.parts.first().map(|p| p.start_ts).unwrap_or_default() as f64
							/ track_state.timescale() as f64,
						segment.duration() as f64 / track_state.timescale() as f64,
						false,
					) {
						self.tasker.submit_task_with_abort(task);
					}
				}
				self.tasker.submit(
					TaskDomain::Normal(rendition),
					format!("part_{part_idx}"),
					upload_media_generator(key, part.data.clone()),
				);
			}
		}

		for idx in track_state
			.retain_segments(config.playlist_segments)
			.into_iter()
			.flat_map(|s| s.parts.into_iter().map(|p| p.idx))
		{
			self.tasker.submit(
				TaskDomain::Normal(rendition),
				format!("delete_part_{idx}"),
				delete_media_generator(video_common::keys::part(
					self.organization_id,
					self.room_id,
					self.connection_id,
					rendition,
					idx,
				)),
			);
		}

		self.update_rendition_manifest(rendition);

		Ok(())
	}

	pub async fn handle_shutdown(
		&mut self,
		global: &Arc<G>,
		mut streams: impl Stream<Item = (io::Result<TrackOut>, Rendition)> + Unpin,
	) -> Result<()> {
		tracing::info!("shutting down transcoder");

		let mut ffmpeg_done = false;

		let result = async {
			loop {
				select! {
					Some(r) = async {
						if !ffmpeg_done {
							Some(self.ffmpeg.process.wait().timeout(Duration::from_secs(2)).await)
						} else {
							None
						}
					} => {
						match r {
							Ok(Ok(status)) => {
								if !status.success() {
									if let Some(mut stderr) = self.ffmpeg.process.stderr.take() {
										let mut buf = Vec::new();
										let size = stderr.read_to_end(&mut buf).await.unwrap_or_default();
										tracing::error!("ffmpeg stdout: {}", String::from_utf8_lossy(&buf[..size]));
									}
								}
								// ffmpeg exited gracefully
							}
							Ok(Err(e)) => {
								tracing::error!(error = %e, "ffmpeg exited with error");
							}
							Err(_) => {
								tracing::error!("ffmpeg timeout while exit");
								self.ffmpeg.process.kill().await.ok();

								if let Some(mut stderr) = self.ffmpeg.process.stderr.take() {
									let mut buf = Vec::new();
									let size = stderr.read_to_end(&mut buf).await.unwrap_or_default();
									tracing::error!("ffmpeg stdout: {}", String::from_utf8_lossy(&buf[..size]));
								}
							}
						}
						ffmpeg_done = true;
					},
					Some(upload) = self.tasker.next_task(global.clone()) => {
						if let Err((task, err)) = upload {
							if task.retry_count() < 5 {
								tracing::error!(error = %err, "failed to upload media");
								self.tasker.requeue(task);
							} else {
								anyhow::bail!("failed to upload media after 5 retries: {}", err);
							}
						}
					},
					Some((result, rendition)) = streams.next() => {
						self.handle_track(global, rendition, result)?;
					},
					else => {
						break;
					}
				}
			}

			Ok::<_, anyhow::Error>(())
		}
		.timeout(Duration::from_secs(5))
		.await;

		match result {
			Ok(Ok(_)) => {}
			Ok(Err(e)) => {
				tracing::error!(error = %e, "failed to shutdown transcoder");
			}
			Err(_) => {
				tracing::error!("timeout during shutdown");
			}
		}

		self.track_state
			.iter_mut()
			.map(|(rendition, track_state)| {
				let Some((segment_idx, part_idx)) = track_state.finish() else {
					return *rendition;
				};

				let key =
					video_common::keys::part(self.organization_id, self.room_id, self.connection_id, *rendition, part_idx);

				let segment = track_state.segment(segment_idx).unwrap();
				let part = segment.part(part_idx).unwrap();

				self.tasker.submit(
					TaskDomain::Normal(*rendition),
					format!("part_{segment_idx}_{part_idx}"),
					upload_media_generator(key, part.data.clone()),
				);

				if let Some(recording) = &mut self.recording {
					if let Some(task) = recording.upload_part(
						*rendition,
						track_state.segment(segment_idx).unwrap().id,
						segment_idx,
						part.data.clone(),
						segment.parts.first().map(|p| p.start_ts).unwrap_or_default() as f64
							/ track_state.timescale() as f64,
						segment.duration() as f64 / track_state.timescale() as f64,
						true,
					) {
						self.tasker.submit_task_with_abort(task);
					}
				}

				*rendition
			})
			.collect::<Vec<_>>()
			.into_iter()
			.for_each(|rendition| {
				self.update_rendition_manifest(rendition);
			});

		while let Some(result) = self.tasker.next_task(global.clone()).await {
			if let Err((task, err)) = result {
				if task.retry_count() < 5 {
					tracing::error!(error = %err, "failed to upload media");
					self.tasker.requeue(task);
				} else {
					tracing::error!("failed to upload media after 5 retries: {}", err);
					break;
				}
			}
		}

		if let Some(shutdown) = self.shutdown.take() {
			match shutdown {
				ingest_watch_response::Shutdown::Stream => {
					// write the playlist states to shutdown
				}
				ingest_watch_response::Shutdown::Transcoder => {
					self.send.try_send(IngestWatchRequest {
						message: Some(ingest_watch_request::Message::Shutdown(
							ingest_watch_request::Shutdown::Complete as i32,
						)),
					})?;
				}
			}
		}

		Ok(())
	}

	fn update_manifest(&mut self) {
		if !self.ingest_ready {
			return;
		}

		let key = video_common::keys::manifest(self.organization_id, self.room_id, self.connection_id);

		let data: Bytes = LiveManifest {
			screenshot_idx: self.screenshot_idx,
		}
		.encode_to_vec()
		.into();

		self.tasker
			.submit_with_abort(TaskDomain::Generic, "manifest", upload_metadata_generator(key, data));
	}

	fn update_rendition_manifest(&mut self, rendition: Rendition) {
		if !self.ingest_ready {
			return;
		}

		let mut info_map = self
			.track_state
			.iter()
			.map(|(rendition, ts)| {
				(
					rendition.to_string(),
					live_rendition_manifest::RenditionInfo {
						next_part_idx: ts.next_part_idx(),
						next_segment_idx: ts.next_segment_idx(),
						next_segment_part_idx: ts.next_segment_part_idx(),
						last_independent_part_idx: ts.last_independent_part_idx(),
					},
				)
			})
			.collect::<HashMap<_, _>>();

		let info = info_map.remove(&rendition.to_string()).unwrap();

		let state = self.track_state.get_mut(&rendition).unwrap();

		let completed = state.complete() && self.shutdown == Some(ingest_watch_response::Shutdown::Stream);

		let manifest = LiveRenditionManifest {
			info: Some(info),
			other_info: info_map,
			completed,
			timescale: state.timescale(),
			total_duration: state.total_duration(),
			recording_data: if let Some(recording) = &self.recording {
				if recording.allow_dvr() {
					Some(live_rendition_manifest::RecordingData {
						recording_ulid: Some(recording.id().into()),
						thumbnails: recording
							.recent_thumbnails()
							.map(|item| live_rendition_manifest::recording_data::RecordingThumbnail {
								idx: item.idx,
								timestamp: item.start_time as f32,
								ulid: Some(item.id.into()),
							})
							.collect(),
					})
				} else {
					None
				}
			} else {
				None
			},
			segments: state
				.segments()
				.map(|s| live_rendition_manifest::Segment {
					idx: s.idx,
					id: Some(s.id.into()),
					parts: s
						.parts
						.iter()
						.map(|p| live_rendition_manifest::Part {
							idx: p.idx,
							duration: p.duration,
							independent: p.independent,
						})
						.collect(),
				})
				.collect(),
		};

		if &manifest == self.manifests.get(&rendition).unwrap() {
			return;
		}

		let data = Bytes::from(manifest.encode_to_vec());

		let key = video_common::keys::rendition_manifest(self.organization_id, self.room_id, self.connection_id, rendition);
		self.tasker.submit_with_abort(
			TaskDomain::Normal(rendition),
			"manifest",
			upload_metadata_generator(key, data),
		);

		self.manifests.insert(rendition, manifest);
	}

	async fn fetch_manifests(&mut self, global: &Arc<G>) -> Result<()> {
		let rendition_manfiests = async {
			futures_util::future::try_join_all(self.track_state.keys().map(|rendition| {
				global
					.metadata_store()
					.get(video_common::keys::rendition_manifest(
						self.organization_id,
						self.room_id,
						self.connection_id,
						*rendition,
					))
					.map_ok(|v| (*rendition, v))
			}))
			.await
		};

		let manifest = async {
			global
				.metadata_store()
				.get(video_common::keys::manifest(
					self.organization_id,
					self.room_id,
					self.connection_id,
				))
				.await
		};

		let (rendition_manfiests, manifest) = try_join!(rendition_manfiests, manifest)?;

		if rendition_manfiests.iter().all(|(_, v)| v.is_none()) && manifest.is_none() {
			return Ok(());
		}

		let Some(manifest) = manifest else {
			anyhow::bail!("missing manifest");
		};

		let manifest = LiveManifest::decode(manifest)?;

		self.screenshot_idx = manifest.screenshot_idx;

		for (rendition, data) in rendition_manfiests {
			let Some(data) = data else {
				anyhow::bail!("missing manifest for rendition {}", rendition);
			};

			let manifest = LiveRenditionManifest::decode(data)?;

			if let Some(recording) = &mut self.recording {
				if let Some(data) = &manifest.recording_data {
					recording.recover_recordings(&data.thumbnails);
				}
			}

			self.track_state.get_mut(&rendition).unwrap().apply_manifest(&manifest);

			self.manifests.insert(rendition, manifest);
		}

		Ok(())
	}
}
