use std::{
    collections::{HashMap, HashSet},
    pin::pin,
};

use crate::{
    hls::{
        self,
        master::{MasterPlaylist, Media},
        media::MediaPlaylist,
    },
    player::{
        fetch::FetchRequest,
        track::{Fragment, ReferenceTrack, TrackResult},
    },
};

use gloo_timers::future::TimeoutFuture;
use mp4::{
    types::{
        ftyp::{FourCC, Ftyp},
        moov::Moov,
    },
    BoxType,
};
use tokio::{
    select,
    sync::{broadcast, mpsc},
};
use url::Url;
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{HtmlVideoElement, MediaSource, SourceBuffer};

use super::{
    blank::VideoFactory,
    inner::PlayerInnerHolder,
    track::{Track, TrackState},
    util::{register_events, Holder},
};

struct SourceBufferHolder {
    sb: Holder<SourceBuffer>,
    rx: mpsc::Receiver<()>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum TrackMapping {
    Audio,
    Video,
    AudioVideo,
    Buffer,
}

impl SourceBufferHolder {
    fn new(media_source: &MediaSource, codec: &str) -> Result<Self, JsValue> {
        let sb = media_source.add_source_buffer(codec)?;
        let (tx, rx) = mpsc::channel(128);

        let cleanup = register_events!(sb, {
            "updateend" => move |_| {
                if tx.try_send(()).is_err() {
                    tracing::warn!("failed to send updateend event");
                }
            }
        });

        Ok(Self {
            sb: Holder::new(sb, cleanup),
            rx,
        })
    }

    fn change_type(&self, codec: &str) -> Result<(), JsValue> {
        self.sb.change_type(codec)?;
        Ok(())
    }

    async fn append_buffer(&mut self, mut data: Vec<u8>) -> Result<(), JsValue> {
        self.sb.append_buffer_with_u8_array(data.as_mut_slice())?;
        self.rx.recv().await;
        Ok(())
    }

    async fn remove(&mut self, start: f64, end: f64) -> Result<(), JsValue> {
        self.sb.remove(start, end)?;
        self.rx.recv().await;
        Ok(())
    }
}

pub struct PlayerRunner {
    inner: PlayerInnerHolder,
    track_states: Vec<TrackState>,

    active_track_id: u32,
    next_track_id: Option<u32>,

    active_reference_track_ids: Vec<u32>,
    fragment_buffer: HashMap<u32, Vec<Fragment>>,

    track_mapping: HashMap<u32, TrackMapping>,

    moov_map: HashMap<u32, Moov>,

    force_mapping: HashMap<TrackMapping, ()>,

    init: bool,
    shutdown_recv: broadcast::Receiver<()>,

    low_latency: bool,

    video: Option<SourceBufferHolder>,
    audio: Option<SourceBufferHolder>,
    audiovideo: Option<SourceBufferHolder>,

    media_source: Holder<MediaSource>,
    video_element: Holder<HtmlVideoElement>,

    video_factory: Option<VideoFactory>,

    evt_recv: mpsc::Receiver<(RunnerEvent, web_sys::Event)>,
}

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
}

fn make_video_holder(
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

fn make_media_source_holder(
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

impl PlayerRunner {
    pub fn new(inner: PlayerInnerHolder, shutdown_recv: broadcast::Receiver<()>) -> Self {
        let ms = MediaSource::new().unwrap();

        let (tx, rx) = mpsc::channel(128);

        let video_element = make_video_holder(inner.aquire().video_element().unwrap(), &tx);
        let media_source = make_media_source_holder(ms, &tx);

        Self {
            inner,
            track_states: Vec::new(),
            shutdown_recv,
            active_track_id: 0,
            next_track_id: None,
            moov_map: HashMap::new(),
            init: false,
            fragment_buffer: HashMap::new(),
            active_reference_track_ids: Vec::new(),
            track_mapping: HashMap::new(),
            force_mapping: HashMap::new(),
            low_latency: false,
            audio: None,
            video: None,
            audiovideo: None,
            media_source,
            video_element,
            video_factory: None,
            evt_recv: rx,
        }
    }

    pub async fn start(mut self) {
        match self.bind_element().await {
            Err(err) => {
                tracing::error!("failed to bind element: {:?}", err);
                self.inner.aquire().send_error(err.into())();
                return;
            }
            Ok(true) => {}
            Ok(false) => return,
        }

        match self.fetch_playlist().await {
            Err(err) => {
                tracing::error!("failed to handle playlist: {:?}", err);
                self.inner.aquire().send_error(err.into())();
                return;
            }
            Ok(true) => {}
            Ok(false) => return,
        }

        self.active_track_id = self.inner.aquire().active_track_id();

        tracing::info!("starting playback");

        for tid in self.active_track_ids() {
            self.track_states.get_mut(tid as usize).unwrap().start();
        }

        'running: loop {
            self.set_low_latency(!self.init);

            let next_track_id = self.inner.aquire().next_track_id();
            if let Some(next_track_id) = next_track_id {
                if Some(next_track_id.track_id()) != self.next_track_id {
                    if self.next_track_id.is_some() {
                        self.active_track_ids()
                            .difference(&self.next_track_ids())
                            .for_each(|tid| {
                                self.track_states
                                    .get_mut(*tid as usize)
                                    .unwrap()
                                    .set_stop_at(None);
                                tracing::trace!("resuming track: {}", tid);
                            });
                        self.next_track_ids()
                            .difference(&self.active_track_ids())
                            .for_each(|tid| {
                                self.track_states.get_mut(*tid as usize).unwrap().stop();
                                tracing::trace!("stopped track: {}", tid);
                            });
                    }

                    if next_track_id.track_id() != self.active_track_id {
                        self.next_track_id = Some(next_track_id.track_id());
                        self.next_track_ids()
                            .difference(&self.active_track_ids())
                            .for_each(|tid| {
                                self.track_states.get_mut(*tid as usize).unwrap().start();
                                tracing::trace!("starting track: {}", tid);
                            });
                    } else {
                        self.next_track_id = None;
                        self.inner.aquire_mut().set_next_track_id(None);
                        self.inner
                            .aquire_mut()
                            .set_active_track_id(self.active_track_id);
                        self.fragment_buffer.clear();
                    }
                }

                if next_track_id.is_force() && self.next_track_id.is_some() {
                    self.active_track_ids()
                        .difference(&self.next_track_ids())
                        .for_each(|tid| {
                            self.track_states.get_mut(*tid as usize).unwrap().stop();
                            tracing::trace!("stopping track: {}", tid);
                            match self.track_mapping.get(tid) {
                                Some(TrackMapping::Audio) => {
                                    self.force_mapping.insert(TrackMapping::Audio, ())
                                }
                                Some(TrackMapping::Video) => {
                                    self.force_mapping.insert(TrackMapping::Video, ())
                                }
                                Some(TrackMapping::AudioVideo) => {
                                    self.force_mapping.insert(TrackMapping::AudioVideo, ())
                                }
                                _ => None,
                            };
                        });
                }
            }

            if let Some(next_track_id) = self.next_track_id {
                if !self
                    .track_states
                    .get(self.active_track_id as usize)
                    .unwrap()
                    .running()
                    || self.active_track_ids().len() == 1
                {
                    self.active_track_id = next_track_id;
                    self.next_track_id = None;
                    self.make_init_seq(None).await.unwrap();
                    self.inner
                        .aquire_mut()
                        .set_active_track_id(self.active_track_id);
                    self.inner.aquire_mut().set_next_track_id(None);
                    tracing::trace!("switched to track: {}", self.active_track_id);
                }
            }

            for tid in self.active_track_ids().union(&self.next_track_ids()) {
                match self.track_states.get_mut(*tid as usize).unwrap().run() {
                    Ok(Some(result)) => match result {
                        TrackResult::Init { moov } => {
                            self.moov_map.insert(*tid, moov);
                            if let Err(err) = self.make_init_seq(Some(*tid)).await {
                                tracing::error!("failed to make init seq: {:?}", err);
                                self.inner.aquire().send_error(err.into())();
                                break 'running;
                            }
                        }
                        TrackResult::Media {
                            fragments,
                            start_time,
                            end_time,
                        } => {
                            if let Err(err) = self
                                .handle_fragments(*tid, fragments, start_time, end_time)
                                .await
                            {
                                tracing::error!("failed to handle media: {:?}", err);
                                self.inner.aquire().send_error(err.into())();
                                break 'running;
                            }
                        }
                    },
                    Ok(None) => {}
                    Err(err) => {
                        tracing::error!("failed to run track: {:?}", err);
                        self.inner.aquire().send_error(err.into())();
                        break 'running;
                    }
                }
            }

            let mut loop_timer = pin!(TimeoutFuture::new(0));
            loop {
                select! {
                    _ = &mut loop_timer => {
                        break;
                    }
                    _ = self.shutdown_recv.recv() => {
                        break 'running;
                    }
                    evt = self.evt_recv.recv() => {
                        tracing::info!("got event: {:?}", evt);
                    }
                }
            }
        }

        tracing::info!("playback stopped");
    }

    async fn handle_fragments(
        &mut self,
        tid: u32,
        fragments: Vec<Fragment>,
        start_time: f64,
        end_time: f64,
    ) -> Result<(), JsValue> {
        if self.next_track_id == Some(tid) {
            // We have the next track data from start_time so we can stop using the old track
            self.active_track_ids()
                .difference(&self.next_track_ids())
                .for_each(|tid| {
                    let track = self.track_states.get_mut(*tid as usize).unwrap();
                    if track.stop_at().is_none() {
                        track.set_stop_at(Some(start_time));
                    }
                });
        }

        tracing::trace!(
            "tid: {} start_time: {} end_time: {}",
            tid,
            start_time,
            end_time
        );

        // If the track is not active we are going to buffer the fragments.
        if !self.track_mapping.contains_key(&tid) {
            return Err(JsValue::from_str(&format!("track: {} is not active", tid)));
        }

        if matches!(self.track_mapping.get(&tid).unwrap(), TrackMapping::Buffer) {
            self.fragment_buffer
                .entry(tid)
                .or_insert_with(Vec::new)
                .extend(fragments);
            return Ok(());
        }

        let mut data = Vec::new();
        fragments.iter().for_each(|fragment| {
            fragment.moof.mux(&mut data).unwrap();
            fragment.mdat.mux(&mut data).unwrap();
        });

        let mut forced = false;

        match self.track_mapping.get(&tid).unwrap() {
            TrackMapping::Audio => {
                if self.force_mapping.remove(&TrackMapping::Audio).is_some() {
                    self.audio.as_mut().unwrap().remove(0.0, start_time).await?;
                    forced = true;
                } else {
                    self.audio
                        .as_mut()
                        .unwrap()
                        .remove(0.0, start_time - 30.0)
                        .await?;
                }
                self.audio.as_mut().unwrap().append_buffer(data).await?;

                if let Some(video_factory) = &mut self.video_factory {
                    let mut data = Vec::new();
                    fragments.iter().for_each(|fragment| {
                        let decode_time = fragment.moof.traf[0]
                            .tfdt
                            .as_ref()
                            .unwrap()
                            .base_media_decode_time;
                        let duration = fragment.moof.traf[0].duration();

                        let (moof, mdat) = video_factory.moof_mdat(decode_time, duration);
                        moof.mux(&mut data).unwrap();
                        mdat.mux(&mut data).unwrap();
                    });

                    self.video.as_mut().unwrap().append_buffer(data).await?;
                }
            }
            TrackMapping::Video => {
                if self.force_mapping.remove(&TrackMapping::Video).is_some() {
                    self.video.as_mut().unwrap().remove(0.0, start_time).await?;
                    forced = true;
                } else {
                    self.video
                        .as_mut()
                        .unwrap()
                        .remove(0.0, start_time - 30.0)
                        .await?;
                }
                self.video.as_mut().unwrap().append_buffer(data).await?;
            }
            TrackMapping::AudioVideo => {
                if self
                    .force_mapping
                    .remove(&TrackMapping::AudioVideo)
                    .is_some()
                {
                    self.audiovideo
                        .as_mut()
                        .unwrap()
                        .remove(0.0, start_time)
                        .await?;
                    forced = true;
                } else {
                    self.audiovideo
                        .as_mut()
                        .unwrap()
                        .remove(0.0, start_time - 30.0)
                        .await?;
                }
                self.audiovideo
                    .as_mut()
                    .unwrap()
                    .append_buffer(data)
                    .await?;
            }
            TrackMapping::Buffer => unreachable!(),
        }

        if forced {
            let current_time = self.inner.aquire().video_element().unwrap().current_time();

            if current_time > start_time && current_time < end_time - 0.1 {
                // Slight hack to push the video forward and prevent it from getting stuck
                self.video_element.set_current_time(current_time + 0.1);
            } else {
                self.inner
                    .aquire()
                    .video_element()
                    .unwrap()
                    .set_current_time(start_time);
            }
        }

        self.autoplay().await;

        Ok(())
    }

    async fn fetch_playlist(&mut self) -> Result<bool, JsValue> {
        let Ok(input_url) = Url::parse(self.inner.aquire().url()) else {
            return Err(JsValue::from_str(&format!("failed to parse url: {}", self.inner.aquire().url())));
        };

        let req = FetchRequest::new("GET", input_url.as_str())
            .header("Accept", "application/vnd.apple.mpegurl")
            .set_timeout(2000)
            .start()?;

        let data = select! {
            r = req.wait_result() => {
                r?
            }
            _ = self.shutdown_recv.recv() => {
                return Ok(false);
            }
        };

        let playlist = match hls::Playlist::try_from(data.as_slice()) {
            Ok(playlist) => playlist,
            Err(err) => return Err(JsValue::from_str(&err)),
        };

        // We now need to determine what kind of playlist we have, if we have a master playlist we need to do some ABR logic to determine what variant to use
        // If we have a media playlist we can just start playing it directly.
        match playlist {
            hls::Playlist::Master(playlist) => self.handle_master_playlist(input_url, playlist)?,
            hls::Playlist::Media(playlist) => self.handle_media_playlist(input_url, playlist)?,
        }

        Ok(true)
    }

    async fn make_init_seq(&mut self, for_tid: Option<u32>) -> Result<(), JsValue> {
        let active_tracks = self.active_track_ids();
        if active_tracks.len() > 2 {
            return Err(JsValue::from_str(
                "too many active tracks, currently only 2 are supported",
            ));
        }

        let next_tracks = self.next_track_ids();
        if next_tracks.len() > 2 {
            return Err(JsValue::from_str(
                "too many next tracks, currently only 2 are supported",
            ));
        }

        tracing::trace!(
            "active_tracks: {:?} next_tracks: {:?}, for_tid: {:?}",
            active_tracks,
            next_tracks,
            for_tid
        );

        let diff = next_tracks
            .difference(&active_tracks)
            .collect::<HashSet<_>>();

        for (tid, moov) in self
            .moov_map
            .clone()
            .iter()
            .filter(|(tid, _)| active_tracks.contains(tid) || next_tracks.contains(tid))
        {
            if let Some(for_tid) = for_tid {
                if *tid != for_tid {
                    continue;
                }
            }

            if diff.contains(tid) {
                self.track_mapping.insert(*tid, TrackMapping::Buffer);
                continue;
            }

            let track = self.track_states.get(*tid as usize).unwrap().track();

            let (sb, mapping) = if moov.traks.is_empty() {
                return Err(JsValue::from_str("no tracks in moov"));
            } else if moov.traks.len() == 1
                && (!track.referenced_group_ids.is_empty() || track.reference.is_some())
            {
                if self.audiovideo.is_some() {
                    return Err(JsValue::from_str("audiovideo track already exists"));
                }

                let trak = moov.traks.get(0).unwrap();
                let codecs = trak.mdia.minf.stbl.stsd.get_codecs().collect::<Vec<_>>();
                if trak.mdia.minf.stbl.stsd.is_audio() {
                    // We have an audio track
                    let codec = format!("audio/mp4; codecs=\"{}\"", &codecs.join(","));
                    if self.audio.is_none() {
                        self.audio = Some(SourceBufferHolder::new(&self.media_source, &codec)?);
                        self.video = Some(SourceBufferHolder::new(
                            &self.media_source,
                            "video/mp4; codecs=\"avc1.4d002a\"",
                        )?);
                    }

                    if self.active_track_ids().len() == 1 {
                        let video_factory = VideoFactory::new(trak.mdia.mdhd.timescale);

                        let codecs = video_factory.moov().traks[0]
                            .mdia
                            .minf
                            .stbl
                            .stsd
                            .get_codecs()
                            .collect::<Vec<_>>();

                        let video = self.video.as_mut().unwrap();
                        video
                            .change_type(&format!("video/mp4; codecs=\"{}\"", codecs.join(",")))?;

                        self.video_factory = Some(video_factory);
                    } else {
                        self.video_factory = None;
                    }

                    let audio = self.audio.as_mut().unwrap();
                    audio.change_type(&codec)?;
                    (audio, TrackMapping::Audio)
                } else if trak.mdia.minf.stbl.stsd.is_video() {
                    // We have a video track
                    let codec = format!("video/mp4; codecs=\"{}\"", &codecs.join(","));
                    if self.video.is_none() {
                        self.video = Some(SourceBufferHolder::new(&self.media_source, &codec)?);
                        self.audio = Some(SourceBufferHolder::new(
                            &self.media_source,
                            "audio/mp4; codecs=\"mp4a.40.2\"",
                        )?);
                    }

                    if self.active_track_ids().len() == 1 {
                        return Err(JsValue::from_str(
                            "video track must be paired with audio track",
                        ));
                    } else {
                        self.video_factory = None;
                    }

                    let video = self.video.as_mut().unwrap();
                    video.change_type(&codec)?;
                    (video, TrackMapping::Video)
                } else {
                    return Err(JsValue::from_str("unsupported track type"));
                }
            } else {
                if self.video.is_some() || self.audio.is_some() {
                    return Err(JsValue::from_str("audio or video track already exists"));
                }

                self.video_factory = None;

                // We have both audio and video tracks
                let audio_trak = moov
                    .traks
                    .iter()
                    .find(|trak| trak.mdia.minf.stbl.stsd.is_audio());
                let video_trak = moov
                    .traks
                    .iter()
                    .find(|trak| trak.mdia.minf.stbl.stsd.is_video());

                if audio_trak.is_none() && video_trak.is_none() {
                    return Err(JsValue::from_str("missing audio and video track"));
                }

                let mut codecs = Vec::new();

                if let Some(audio_trak) = audio_trak {
                    let audio_codecs = audio_trak.mdia.minf.stbl.stsd.get_codecs();
                    codecs.extend(audio_codecs);
                }

                if let Some(video_trak) = video_trak {
                    let video_codecs = video_trak.mdia.minf.stbl.stsd.get_codecs();
                    codecs.extend(video_codecs);
                }

                let codec = format!("video/mp4; codecs=\"{}\"", &codecs.join(","));

                if self.audiovideo.is_none() {
                    self.audiovideo = Some(SourceBufferHolder::new(&self.media_source, &codec)?);
                }

                let audiovideo = self.audiovideo.as_mut().unwrap();
                audiovideo.change_type(&codec)?;
                (audiovideo, TrackMapping::AudioVideo)
            };

            // Construct a moov segment
            let mut data = Vec::new();
            Ftyp::new(FourCC::Iso5, 512, vec![FourCC::Iso5, FourCC::Iso6])
                .mux(&mut data)
                .unwrap();
            moov.mux(&mut data).unwrap();

            sb.append_buffer(data).await?;

            if let Some(video_factory) = &self.video_factory {
                let mut data = Vec::new();
                Ftyp::new(FourCC::Iso5, 512, vec![FourCC::Iso5, FourCC::Iso6])
                    .mux(&mut data)
                    .unwrap();
                video_factory.moov().mux(&mut data).unwrap();

                self.video.as_mut().unwrap().append_buffer(data).await?;
            }

            if matches!(
                self.track_mapping.insert(*tid, mapping),
                Some(TrackMapping::Buffer)
            ) {
                let fragments = self.fragment_buffer.remove(tid).unwrap_or_default();
                let start_time = fragments.first().map(|f| f.start_time).unwrap_or_default();
                let end_time = fragments.last().map(|f| f.end_time).unwrap_or_default();

                self.handle_fragments(*tid, fragments, start_time, end_time)
                    .await?;
            }
        }

        Ok(())
    }

    async fn autoplay(&mut self) {
        if self.init {
            return;
        }

        let fut = {
            let inner = self.inner.aquire();
            let element = inner.video_element().unwrap();
            let Ok(start) = element.buffered().start(0) else {
                return;
            };

            self.init = true;

            element.set_current_time(start);
            element.play().map(JsFuture::from)
        };

        if let Ok(fut) = fut {
            fut.await.ok();
        }
    }

    async fn bind_element(&mut self) -> Result<bool, JsValue> {
        let url = web_sys::Url::create_object_url_with_source(&self.media_source)?;

        self.video_element.set_src(&url);

        let mut result = Ok(true);

        let mut global_evt = self.shutdown_recv.resubscribe();

        'l: loop {
            select! {
                _ = global_evt.recv() => {
                    result = Ok(false);
                    break 'l;
                }
                evt = self.evt_recv.recv() => {
                    match evt {
                        Some((RunnerEvent::MediaSourceOpen, _)) => {
                            break 'l;
                        }
                        Some((RunnerEvent::MediaSourceClose, _)) => {
                            result = Err(JsValue::from_str("media source closed"));
                            break 'l;
                        }
                        Some((RunnerEvent::MediaSourceEnded, _)) => {
                            result = Err(JsValue::from_str("media source ended"));
                            break 'l;
                        }
                        None => unreachable!(),
                        _ => {}
                    }
                }
            }
        }

        web_sys::Url::revoke_object_url(&url)?;

        result
    }

    fn set_low_latency(&mut self, force: bool) {
        let low_latency = self.inner.aquire().low_latency();
        if self.low_latency != low_latency || force {
            self.low_latency = low_latency;
            self.track_states.iter_mut().for_each(|track| {
                track.set_low_latency(low_latency);
            });

            let buffered = self.inner.aquire().video_element().unwrap().buffered();
            if buffered.length() != 0 {
                self.inner
                    .aquire()
                    .video_element()
                    .unwrap()
                    .set_current_time(
                        (if low_latency {
                            buffered
                                .end(buffered.length() - 1)
                                .map(|t| t - 0.1)
                                .unwrap_or_default()
                        } else {
                            buffered
                                .end(buffered.length() - 1)
                                .map(|t| t - 2.0)
                                .unwrap_or_default()
                        })
                        .max(0.0),
                    )
            }
        }
    }

    fn active_track_ids(&self) -> HashSet<u32> {
        self.track_ids(self.active_track_id)
    }

    fn next_track_ids(&self) -> HashSet<u32> {
        self.next_track_id
            .map(|id| self.track_ids(id))
            .unwrap_or_default()
    }

    fn track_ids(&self, track_id: u32) -> HashSet<u32> {
        let active_track = self.track_states.get(track_id as usize).unwrap();

        let mut track_ids = active_track
            .track()
            .referenced_group_ids
            .iter()
            .map(|id| *self.active_reference_track_ids.get(*id as usize).unwrap())
            .collect::<HashSet<_>>();

        track_ids.insert(track_id);

        track_ids
    }

    fn handle_master_playlist(
        &mut self,
        input_url: Url,
        mut playlist: MasterPlaylist,
    ) -> Result<(), JsValue> {
        let mut inner = self.inner.aquire_mut();

        let mut reference_streams = HashSet::new();

        for stream in playlist.streams.iter() {
            if let Some(audio) = stream.audio.as_ref() {
                reference_streams.insert(audio);
            }

            if let Some(video) = stream.video.as_ref() {
                reference_streams.insert(video);
            }
        }

        let mut m3u8_url_to_track = HashMap::new();

        enum TrackReference {
            Flat(Media),
            Reference(u32),
        }

        let mut reference_tracks = HashMap::new();
        let mut current_track_idx = 0;
        let mut group_id = 0;

        for stream in reference_streams.into_iter() {
            let Some(groups) = playlist.groups.get_mut(stream) else {
                return Err(JsValue::from_str(&format!("failed to find group for stream: {}", stream)));
            };

            let pos = groups.iter().position(|item| item.default).unwrap_or(0);
            groups.iter_mut().for_each(|item| item.default = false);

            let default_item = &mut groups[pos];
            if default_item.uri.is_empty() {
                // This is a reference track but is not really a reference track
                reference_tracks.insert(stream.clone(), TrackReference::Flat(default_item.clone()));
                continue;
            }

            default_item.default = true;

            // Otherwise this is actually a reference track
            // So we need to generate a new track id for it.
            let mut ids = HashSet::new();
            for track in groups {
                let url = match Url::parse(&track.uri).or_else(|_| input_url.join(&track.uri)) {
                    Ok(url) => url,
                    Err(err) => {
                        return Err(JsValue::from_str(&format!("failed to parse url: {}", err)));
                    }
                };

                let track_id = m3u8_url_to_track
                    .entry(url.clone())
                    .or_insert_with(|| {
                        let t = Track {
                            id: current_track_idx,
                            is_variant_track: false,
                            playlist_url: url.clone(),
                            referenced_group_ids: Vec::new(),
                            name: Some(track.name.clone()),
                            bandwidth: None,
                            codecs: None,
                            frame_rate: None,
                            height: None,
                            width: None,
                            reference: Some(ReferenceTrack {
                                group_id: group_id as u32,
                                is_default: track.default,
                            }),
                        };

                        current_track_idx += 1;

                        t
                    })
                    .id;

                if track.default {
                    self.active_reference_track_ids.push(track_id);
                }

                ids.insert(track_id);
            }

            reference_tracks.insert(stream.clone(), TrackReference::Reference(group_id as u32));
            group_id += 1;
        }

        for stream in playlist.streams.iter() {
            let url = match Url::parse(&stream.uri).or_else(|_| input_url.join(&stream.uri)) {
                Ok(url) => url,
                Err(err) => {
                    return Err(JsValue::from_str(&format!("failed to parse url: {}", err)));
                }
            };

            let track = m3u8_url_to_track.entry(url.clone()).or_insert_with(|| {
                let t = Track {
                    id: current_track_idx,
                    is_variant_track: true,
                    playlist_url: url.clone(),
                    referenced_group_ids: Vec::new(),
                    name: None,
                    reference: None,
                    bandwidth: Some(stream.bandwidth),
                    codecs: stream.codecs.clone(),
                    frame_rate: stream.frame_rate,
                    width: stream.resolution.map(|r| r.0),
                    height: stream.resolution.map(|r| r.1),
                };

                current_track_idx += 1;

                t
            });

            track.bandwidth = Some(stream.bandwidth);
            track.codecs = stream.codecs.clone();
            track.frame_rate = stream.frame_rate;
            track.width = stream.resolution.map(|r| r.0);
            track.height = stream.resolution.map(|r| r.1);
            track.is_variant_track = true;

            if let Some(audio) = stream.audio.as_ref() {
                match reference_tracks.get(audio) {
                    Some(TrackReference::Flat(media)) => {
                        track.name = Some(media.name.clone());
                    }
                    Some(TrackReference::Reference(group_id)) => {
                        if track.reference.as_ref().map(|t| t.group_id) != Some(*group_id) {
                            track.referenced_group_ids.push(*group_id);
                        }
                    }
                    None => {
                        return Err(JsValue::from_str(&format!(
                            "failed to find reference track for audio: {}",
                            audio
                        )));
                    }
                }
            }

            if let Some(video) = stream.video.as_ref() {
                match reference_tracks.get(video) {
                    Some(TrackReference::Flat(media)) => {
                        track.name = Some(media.name.clone());
                    }
                    Some(TrackReference::Reference(group_id)) => {
                        if track.reference.as_ref().map(|t| t.group_id) != Some(*group_id) {
                            track.referenced_group_ids.push(*group_id);
                        }
                    }
                    None => {
                        return Err(JsValue::from_str(&format!(
                            "failed to find reference track for video: {}",
                            video
                        )));
                    }
                }
            }
        }

        let mut tracks = m3u8_url_to_track.into_values().collect::<Vec<_>>();
        tracks.sort_by(|a, b| a.id.cmp(&b.id));

        self.track_states = tracks.clone().into_iter().map(TrackState::new).collect();

        let fire_event = inner.set_tracks(tracks, true);
        inner.set_active_reference_track_ids(self.active_reference_track_ids.clone());
        inner.set_active_track_id(0);

        drop(inner);

        fire_event();

        Ok(())
    }

    fn handle_media_playlist(
        &mut self,
        input_url: Url,
        playlist: MediaPlaylist,
    ) -> Result<(), JsValue> {
        let mut inner = self.inner.aquire_mut();

        let track = Track {
            id: 0,
            bandwidth: None,
            is_variant_track: true,
            name: None,
            playlist_url: input_url,
            referenced_group_ids: Vec::new(),
            reference: None,
            codecs: None,
            frame_rate: None,
            height: None,
            width: None,
        };

        let mut track_state = TrackState::new(track.clone());
        track_state.set_playlist(playlist);

        self.track_states = vec![track_state];

        let fire_event = inner.set_tracks(vec![track], false);
        inner.set_active_track_id(0);

        drop(inner);
        fire_event();

        Ok(())
    }
}
