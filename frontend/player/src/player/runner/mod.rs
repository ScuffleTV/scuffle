use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    pin::pin,
};

use crate::{
    hls::{self, master::MasterPlaylist, media::MediaPlaylist},
    player::{
        events::{EventError, EventLoad, UserEvent},
        fetch::FetchRequest,
        inner::NextVariant,
        runner::source_buffer::SourceBufferHolder,
        track::{Fragment, TrackResult},
        util::now,
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
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Document, HtmlVideoElement, MediaSource};

mod events;
mod source_buffer;
mod util;

use self::{
    events::RunnerEvent,
    source_buffer::SourceBuffers,
    util::{make_document_holder, make_media_source_holder, make_video_holder},
};

use super::{
    bandwidth::Bandwidth,
    blank::VideoFactory,
    events::EventAbrChange,
    inner::PlayerInnerHolder,
    track::{Track, TrackState, Variant},
    util::Holder,
};

pub struct PlayerRunner {
    inner: PlayerInnerHolder,

    /// Track states (offset by track id)
    track_states: Vec<TrackState>,
    /// Variants (offset by variant id)
    variants: Vec<Variant>,

    active_variant_id: u32,
    next_variant_id: Option<u32>,

    /// Group Id -> Track Id
    active_group_track_ids: Vec<u32>,

    /// Track Id -> Vec<Fragment>
    /// This is used to buffer fragments for tracks that are not active
    fragment_buffer: HashMap<u32, Vec<Fragment>>,

    /// Track Id -> Moov
    /// This is a map of the last moov we got for a track, this is used to generate init segments
    moov_map: HashMap<u32, Moov>,

    /// If we have initialized the player
    init: bool,

    /// Used to shutdown all the tasks
    shutdown_recv: broadcast::Receiver<()>,

    /// If we are in low latency mode
    low_latency: bool,

    /// The source buffers
    source_buffers: SourceBuffers,

    /// The media source
    media_source: Holder<MediaSource>,

    /// The video element
    video_element: Holder<HtmlVideoElement>,

    /// The dom document
    document: Holder<Document>,

    /// Video factory, used to generate black video frames
    video_factory: Option<VideoFactory>,

    /// Last iteration time
    last_iteration: f64,
    /// Last time we switched variants due to ABR
    last_abr_switch: f64,

    /// The moment the document was hidden and what variant was active at that time (since when we switch to the background we switch to audio only)
    document_hidden_at: Option<f64>,
    // The variant id that was active when we switched to audio only mode.
    document_hidden_variant_id: Option<u32>,

    /// The bandwidth estimator (used for ABR)
    bandwidth: Bandwidth,

    /// Last ABR Bandwidth Calculation
    last_abr_bandwidth: Option<u32>,

    /// The event receiver
    evt_recv: mpsc::Receiver<(RunnerEvent, web_sys::Event)>,
}

impl PlayerRunner {
    pub fn new(inner: PlayerInnerHolder, shutdown_recv: broadcast::Receiver<()>) -> Self {
        let ms = MediaSource::new().unwrap();

        let (tx, rx) = mpsc::channel(128);

        let video_element = make_video_holder(inner.video_element().unwrap(), &tx);
        let media_source = make_media_source_holder(ms, &tx);
        let document = make_document_holder(&tx);

        Self {
            track_states: Vec::new(),
            shutdown_recv,
            active_variant_id: 0,
            next_variant_id: None,
            moov_map: HashMap::new(),
            init: false,
            fragment_buffer: HashMap::new(),
            variants: Vec::new(),
            active_group_track_ids: Vec::new(),
            low_latency: false,
            source_buffers: SourceBuffers::None,
            media_source,
            video_element,
            document_hidden_at: None,
            document_hidden_variant_id: None,
            document,
            video_factory: None,
            evt_recv: rx,
            last_iteration: 0.0,
            last_abr_switch: 0.0,
            bandwidth: Bandwidth::new(),
            last_abr_bandwidth: None,
            inner,
        }
    }

    pub async fn start(mut self) {
        match self.bind_element().await {
            Err(err) => {
                tracing::error!("failed to bind element: {:?}", err);
                self.inner.emit_event(EventError::from(err));
                return;
            }
            Ok(true) => {}
            Ok(false) => return,
        }

        match self.fetch_playlist().await {
            Err(err) => {
                tracing::error!("failed to handle playlist: {:?}", err);
                self.inner.emit_event(EventError::from(err));
                return;
            }
            Ok(true) => {}
            Ok(false) => return,
        }

        self.active_variant_id = self.inner.active_variant_id();

        let url = self.inner.url();
        self.inner.emit_event(EventLoad { url });

        tracing::info!("starting playback");

        for tid in self.active_track_ids() {
            self.track_states.get_mut(tid as usize).unwrap().start();
        }

        self.last_iteration = now();

        'running: loop {
            self.set_low_latency(!self.init);

            if self.document.hidden() && self.document_hidden_at.is_none() {
                self.document_hidden_at = Some(now());
            } else if !self.document.hidden() && self.document_hidden_variant_id.is_some() {
                self.document_hidden_at = None;
                let mut variant_id = self.document_hidden_variant_id.take().unwrap();
                if self.active_variant_id != variant_id {
                    if self.inner.abr_enabled() {
                        variant_id = self.abr_variant_id().unwrap_or(variant_id);
                    }

                    self.inner
                        .set_next_variant_id(Some(NextVariant::Force(variant_id)));
                }
            } else if self.document.hidden()
                && self.document_hidden_at.is_some()
                && self.document_hidden_variant_id.is_none()
            {
                if let Some(document_hidden_at) = self.document_hidden_at {
                    let audio_variant = self
                        .variants
                        .iter()
                        .find(|v| v.video_track.is_none())
                        .unwrap()
                        .id;

                    if now() - document_hidden_at > 5000.0
                        && self.next_variant_id.is_none()
                        && self.active_variant_id != audio_variant
                    {
                        self.document_hidden_variant_id = Some(self.active_variant_id);
                        self.inner
                            .set_next_variant_id(Some(NextVariant::Switch(audio_variant)))
                    }
                }
            }

            let current_time = now();
            let delta = current_time - self.last_iteration;
            self.last_iteration = current_time;
            // If the delta was bigger than 500ms we need to seek forward because there was likely a player stall (if we were playing)

            if self.document_hidden_at.is_none() {
                self.inner.video_element().and_then(|el| {
                    if el.paused() || !self.low_latency {
                        return None;
                    }

                    let buffered_range = el.buffered();
                    let length = buffered_range.length();
                    if length == 0 {
                        return None;
                    }

                    let end = buffered_range.end(length - 1).unwrap();

                    let delta = end - el.current_time();
                    if delta < 1.5 {
                        return None;
                    }

                    el.set_current_time(end - 0.5);

                    Some(())
                });
            }

            // If we took more then 5s to loop we need to refresh the playlists before requesting fragments.
            if delta > 2000.0 {
                self.active_track_ids()
                    .union(&self.next_track_ids())
                    .collect::<HashSet<_>>()
                    .into_iter()
                    .for_each(|tid| {
                        self.track_states.get_mut(*tid as usize).unwrap().stop();
                    });

                let nvid = self.inner.next_variant_id();
                if let Some(nvid) = nvid {
                    self.inner.set_next_variant_id(None);
                    self.inner.set_active_variant_id(nvid.variant_id());
                    self.active_variant_id = nvid.variant_id();
                }

                self.active_track_ids().into_iter().for_each(|tid| {
                    self.track_states.get_mut(tid as usize).unwrap().start();
                });

                tracing::info!("refreshing playlists");
            }

            if self.inner.abr_enabled()
                && self.inner.next_variant_id().is_none()
                && self.document_hidden_at.is_none()
            {
                if let Some(variant_id) = self.abr_variant_id() {
                    if variant_id != self.active_variant_id && self.last_abr_switch + 5000.0 < now()
                    {
                        self.last_abr_switch = now();
                        self.inner
                            .set_next_variant_id(Some(NextVariant::Switch(variant_id)));
                    }
                }
            }

            if let Some(next_variant_id) = self.inner.next_variant_id() {
                if Some(next_variant_id.variant_id()) != self.next_variant_id {
                    if self.next_variant_id.is_some() {
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

                    self.fragment_buffer.clear();

                    if next_variant_id.variant_id() != self.active_variant_id {
                        self.next_variant_id = Some(next_variant_id.variant_id());
                        self.next_track_ids()
                            .difference(&self.active_track_ids())
                            .for_each(|tid| {
                                tracing::trace!("starting track: {}", tid);
                                let new_track_url =
                                    self.track_states.get(*tid as usize).unwrap().url();
                                if self.low_latency && !next_variant_id.is_force() {
                                    if let Some(id) = self.active_track_ids().iter().next() {
                                        if let Some(report) = self
                                            .track_states
                                            .get(*id as usize)
                                            .unwrap()
                                            .rendition_reports(new_track_url)
                                        {
                                            tracing::info!("rendition report: {:?}", report);
                                            self.track_states
                                                .get_mut(*tid as usize)
                                                .unwrap()
                                                .start_at(
                                                    report.last_msn
                                                        + if report.last_part == 0 { 0 } else { 1 },
                                                );
                                            return;
                                        }
                                    }
                                }

                                self.track_states.get_mut(*tid as usize).unwrap().start();
                            });
                    } else {
                        self.next_variant_id = None;
                        self.inner.set_next_variant_id(None);
                        self.inner.set_active_variant_id(self.active_variant_id);
                    }

                    // Audio only case
                    if self.next_track_ids().len() == 1 {
                        self.active_track_ids()
                            .difference(&self.next_track_ids())
                            .for_each(|t| self.track_states[*t as usize].stop());
                    }

                    if next_variant_id.is_force() && self.next_variant_id.is_some() {
                        self.active_track_ids()
                            .difference(&self.next_track_ids())
                            .for_each(|tid| {
                                self.track_states.get_mut(*tid as usize).unwrap().stop();
                                tracing::trace!("stopping track: {}", tid);
                            });
                    }
                }
            }

            if let Some(next_variant_id) = self.next_variant_id {
                let next_variant = self.variants.get(next_variant_id as usize).unwrap();
                let current_variant = self.variants.get(self.active_variant_id as usize).unwrap();

                let next_video_track_id = next_variant
                    .video_track
                    .map(|id| self.active_group_track_ids[id as usize] as usize);

                let current_video_track_id = current_variant
                    .video_track
                    .map(|id| self.active_group_track_ids[id as usize] as usize);

                if next_video_track_id != current_video_track_id
                    && !current_video_track_id
                        .map(|id| self.track_states[id].running())
                        .unwrap_or_else(|| self.video_factory.is_some())
                {
                    self.active_track_ids()
                        .difference(&self.next_track_ids())
                        .for_each(|tid| {
                            self.track_states.get_mut(*tid as usize).unwrap().stop();
                            tracing::trace!("stopping track: {}", tid);
                        });

                    self.active_variant_id = next_variant_id;
                    self.next_variant_id = None;
                    self.make_init_seq(None).await.unwrap();
                    self.inner.set_active_variant_id(self.active_variant_id);
                    self.inner.set_next_variant_id(None);

                    tracing::trace!("switched to variant: {}", self.active_variant_id);
                }
            }

            for tid in self.active_track_ids().union(&self.next_track_ids()) {
                match self.track_states.get_mut(*tid as usize).unwrap().run() {
                    Ok(Some(result)) => match result {
                        TrackResult::Init { moov } => {
                            self.moov_map.insert(*tid, moov);
                            if let Err(err) = self.make_init_seq(Some(*tid)).await {
                                tracing::error!("failed to make init seq: {:?}", err);
                                self.inner.emit_event(EventError::from(err));
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
                                self.inner.emit_event(EventError::from(err));
                                break 'running;
                            }
                        }
                    },
                    Ok(None) => {}
                    Err(err) => {
                        tracing::error!("failed to run track: {:?}", err);
                        self.inner.emit_event(EventError::from(err));
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
                        let (evt, js_evt) = evt.unwrap();
                        match evt {
                            RunnerEvent::VideoError => {
                                tracing::error!("video error: {:?}", js_evt);
                                self.inner.emit_event(EventError::from(JsValue::from(js_evt)));
                                break 'running;
                            }
                            RunnerEvent::DocumentVisibilityChange => {
                                if self.document.hidden() {
                                    self.document_hidden_at = Some(now());
                                } else if self.document_hidden_at.is_some() {
                                    self.document_hidden_at = None;
                                    if let Some(mut variant_id) = self.document_hidden_variant_id.take() {
                                        if self.active_variant_id != variant_id {
                                            if self.inner.abr_enabled() {
                                                variant_id = self.abr_variant_id().unwrap_or(variant_id);
                                            }

                                            self.inner.set_next_variant_id(Some(NextVariant::Force(variant_id)));
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        tracing::info!("playback stopped");
        self.inner.emit_event(UserEvent::Shutdown);
    }

    fn abr_variant_id(&mut self) -> Option<u32> {
        let bandwidth = self.bandwidth.get()? * 8;

        // We have some bandwidth estimation, so we should try do some ABR to get the best quality
        let best_variant = self.variants.iter().find(|v| {
            let bandwidth = match self.active_variant_id.cmp(&v.id) {
                // Discourage switching to a higher quality unless we can get a 25% increase in bandwidth
                Ordering::Greater => v.bandwidth as f64 * 1.5 <= bandwidth as f64,
                // Incourage staying on the same quality unless our bandwidth is 5% lower then the current quality
                Ordering::Equal => v.bandwidth as f64 * 0.95 <= bandwidth as f64,
                // Incourage switching to a lower quality.
                Ordering::Less => v.bandwidth as f64 <= bandwidth as f64,
            };

            bandwidth && v.audio_track.is_some() && v.video_track.is_some()
        });

        if self.last_abr_bandwidth != Some(bandwidth) {
            self.last_abr_bandwidth = Some(bandwidth);
            self.inner.emit_event(EventAbrChange {
                enabled: true,
                variant_id: best_variant.map(|v| v.id),
                bandwidth: Some(bandwidth),
            });
        }

        Some(best_variant?.id)
    }

    async fn handle_fragments(
        &mut self,
        tid: u32,
        fragments: Vec<Fragment>,
        start_time: f64,
        end_time: f64,
    ) -> Result<(), JsValue> {
        if !self.active_track_ids().contains(&tid) && self.next_track_ids().contains(&tid) {
            tracing::debug!("Stopping old tracks");
            // We have the next track data from start_time so we can stop using the old track
            self.active_track_ids()
                .difference(&self.next_track_ids())
                .for_each(|tid| {
                    let track = self.track_states.get_mut(*tid as usize).unwrap();
                    if track.stop_at().is_none() {
                        track.set_stop_at(Some(start_time));
                    }
                });

            if let Some(video_factory) = &mut self.video_factory {
                video_factory.set_stop_at(Some(start_time));
            }
        }

        tracing::trace!(
            "tid: {} start_time: {} end_time: {}",
            tid,
            start_time,
            end_time
        );

        if !self.active_track_ids().contains(&tid) && self.next_track_ids().contains(&tid) {
            tracing::debug!(
                "Buffering fragments for track: {} ({} - {})",
                tid,
                start_time,
                end_time
            );
            self.fragment_buffer
                .entry(tid)
                .or_insert_with(Vec::new)
                .extend(fragments);
            return Ok(());
        } else if !self.active_track_ids().contains(&tid) {
            tracing::warn!(
                "Got fragments for track that is not active or next: {}",
                tid
            );
            return Ok(());
        }

        let mut data = Vec::new();
        fragments.iter().for_each(|fragment| {
            fragment.moof.mux(&mut data).unwrap();
            fragment.mdat.mux(&mut data).unwrap();
        });

        let buffer = match &mut self.source_buffers {
            SourceBuffers::AudioVideoCombined(av) => av,
            SourceBuffers::AudioVideoSplit { audio, video } => {
                let variant = self.variants.get(self.active_variant_id as usize).unwrap();
                if let Some(group_id) = variant.audio_track {
                    if self.active_group_track_ids[group_id as usize] == tid {
                        audio
                    } else {
                        video
                    }
                } else {
                    video
                }
            }
            SourceBuffers::None => {
                return Err(JsValue::from_str("no source buffers"));
            }
        };

        let current_time = self.inner.video_element().unwrap().current_time();
        let buffered_ranges = buffer.buffered().unwrap();
        let duration = if buffered_ranges.length() != 0 {
            buffered_ranges.end(buffered_ranges.length() - 1).unwrap()
        } else {
            0.0
        };

        // We have already buffered some data in this region so we must remove it.
        buffer.remove(0.0, current_time - 30.0).await?;
        buffer.remove(start_time, duration).await?;
        buffer.append_buffer(data).await?;

        // If we have a video factory we must be in split mode and also we need to buffer black frames
        if let Some(video_factory) = &mut self.video_factory {
            let mut data = Vec::new();
            tracing::trace!(
                "Generating black frames for track: {} ({} - {})",
                tid,
                start_time,
                end_time
            );

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

            self.source_buffers
                .video()
                .unwrap()
                .append_buffer(data)
                .await?;

            if let Some(stop_at) = video_factory.stop_at() {
                if end_time >= stop_at {
                    tracing::info!("stopping video factory");
                    self.video_factory = None;
                }
            }
        }

        TimeoutFuture::new(0).await;

        let buffered_ranges = self.inner.video_element().unwrap().buffered();
        if buffered_ranges.length() != 0 {
            let current_time = self.inner.video_element().unwrap().current_time();
            let last_range = (
                buffered_ranges.start(buffered_ranges.length() - 1).unwrap(),
                buffered_ranges.end(buffered_ranges.length() - 1).unwrap(),
            );
            if last_range.0 > current_time {
                tracing::info!(
                    "seeking ahead to last buffered range: {} - {:?}",
                    current_time,
                    last_range
                );
                self.inner
                    .video_element()
                    .unwrap()
                    .set_current_time(last_range.0);
            } else if last_range.1 < current_time {
                tracing::info!(
                    "seeking back to last buffered range: {} - {:?}",
                    current_time,
                    last_range
                );
                self.inner
                    .video_element()
                    .unwrap()
                    .set_current_time(last_range.1);
            }
        }

        self.autoplay().await;

        Ok(())
    }

    async fn fetch_playlist(&mut self) -> Result<bool, JsValue> {
        let Ok(input_url) = Url::parse(&self.inner.url()) else {
            return Err(JsValue::from_str(&format!("failed to parse url: {}", self.inner.url())));
        };

        let mut req = FetchRequest::new("GET", input_url.clone())
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
                continue;
            }

            let sb = if moov.traks.is_empty() {
                return Err(JsValue::from_str("no tracks in moov"));
            } else if moov.traks.len() == 1 {
                if self.source_buffers.audiovideo().is_some() {
                    return Err(JsValue::from_str("audiovideo track already exists"));
                }

                let trak = moov.traks.get(0).unwrap();
                let codecs = trak.mdia.minf.stbl.stsd.get_codecs().collect::<Vec<_>>();
                if trak.mdia.minf.stbl.stsd.is_audio() {
                    // We have an audio track
                    let codec = format!("audio/mp4; codecs=\"{}\"", &codecs.join(","));
                    if self.source_buffers.audio().is_none() {
                        self.source_buffers = SourceBuffers::AudioVideoSplit {
                            audio: SourceBufferHolder::new(&self.media_source, &codec)?,
                            video: SourceBufferHolder::new(
                                &self.media_source,
                                "video/mp4; codecs=\"avc1.4d002a\"", // This is a generic codec we will change it later when we have more information about the video track
                            )?,
                        };
                    }

                    // If we only have 1 track but we are using a split source buffer we need to make a dummy video track
                    // We use video factory to generate dummy frames.
                    if self.active_track_ids().len() == 1 {
                        let video_factory = VideoFactory::new(trak.mdia.mdhd.timescale);

                        let codecs = video_factory.moov().traks[0]
                            .mdia
                            .minf
                            .stbl
                            .stsd
                            .get_codecs()
                            .collect::<Vec<_>>();

                        self.source_buffers
                            .video()
                            .unwrap()
                            .change_type(&format!("video/mp4; codecs=\"{}\"", codecs.join(",")))?;

                        tracing::info!("using video factory to generate dummy video track");

                        self.video_factory = Some(video_factory);
                    }

                    let audio = self.source_buffers.audio().unwrap();
                    audio.change_type(&codec)?;
                    audio
                } else if trak.mdia.minf.stbl.stsd.is_video() {
                    // We have a video track
                    let codec = format!("video/mp4; codecs=\"{}\"", &codecs.join(","));
                    if self.source_buffers.video().is_none() {
                        self.source_buffers = SourceBuffers::AudioVideoSplit {
                            audio: SourceBufferHolder::new(
                                &self.media_source,
                                "audio/mp4; codecs=\"mp4a.40.2\"",
                            )?,
                            video: SourceBufferHolder::new(&self.media_source, &codec)?,
                        };
                    }

                    if self.active_track_ids().len() == 1 {
                        return Err(JsValue::from_str(
                            "video track must be paired with audio track",
                        ));
                    }

                    // Since we have a real video track we don't need the video factory anymore
                    self.video_factory = None;

                    let video = self.source_buffers.video().unwrap();
                    video.change_type(&codec)?;
                    video
                } else {
                    return Err(JsValue::from_str("unsupported track type"));
                }
            } else {
                if self.source_buffers.audio().is_some() || self.source_buffers.video().is_some() {
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

                if self.source_buffers.audiovideo().is_none() {
                    self.source_buffers = SourceBuffers::AudioVideoCombined(
                        SourceBufferHolder::new(&self.media_source, &codec)?,
                    );
                }

                let audiovideo = self.source_buffers.audiovideo().unwrap();
                audiovideo.change_type(&codec)?;
                audiovideo
            };

            // Construct a moov segment
            let mut data = Vec::new();
            Ftyp::new(FourCC::Iso5, 512, vec![FourCC::Iso5, FourCC::Iso6])
                .mux(&mut data)
                .unwrap();
            moov.mux(&mut data).unwrap();

            sb.append_buffer(data).await?;

            // If we have a video factory we need to generate a dummy init segment
            if let Some(video_factory) = &self.video_factory {
                let mut data = Vec::new();
                Ftyp::new(FourCC::Iso5, 512, vec![FourCC::Iso5, FourCC::Iso6])
                    .mux(&mut data)
                    .unwrap();
                video_factory.moov().mux(&mut data).unwrap();

                self.source_buffers
                    .video()
                    .unwrap()
                    .append_buffer(data)
                    .await?;
            }

            // If we had buffered fragments we need to handle them now
            if let Some(fragments) = self.fragment_buffer.remove(tid) {
                let start_time = fragments.first().map(|f| f.start_time).unwrap_or_default();
                let end_time = fragments.last().map(|f| f.end_time).unwrap_or_default();

                tracing::debug!("Handling buffered fragments: {} - {}", start_time, end_time);

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

        let element = self.inner.video_element().unwrap();
        let buffered = element.buffered();
        if buffered.length() == 0 {
            return;
        }

        let Ok(start) = buffered.start(buffered.length() - 1) else {
            return;
        };

        self.init = true;

        element.set_current_time(start);

        if let Ok(fut) = element.play().map(JsFuture::from) {
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
        let low_latency = self.inner.low_latency();
        if self.low_latency != low_latency || force {
            self.low_latency = low_latency;
            self.track_states.iter_mut().for_each(|track| {
                track.set_low_latency(low_latency);
            });

            let buffered = self.inner.video_element().unwrap().buffered();
            if buffered.length() != 0 {
                self.inner.video_element().unwrap().set_current_time(
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

        self.bandwidth
            .set_max_count(if low_latency { 15 } else { 5 })
    }

    fn active_track_ids(&self) -> HashSet<u32> {
        self.track_ids(self.active_variant_id)
    }

    fn next_track_ids(&self) -> HashSet<u32> {
        self.next_variant_id
            .map(|id| self.track_ids(id))
            .unwrap_or_default()
    }

    fn track_ids(&self, variant_id: u32) -> HashSet<u32> {
        let Some(active_track) = self.variants.get(variant_id as usize) else {
            return HashSet::new();
        };

        active_track
            .audio_track
            .iter()
            .chain(active_track.video_track.iter())
            .map(|id| self.active_group_track_ids[*id as usize])
            .collect::<HashSet<_>>()
    }

    fn handle_master_playlist(
        &mut self,
        input_url: Url,
        mut playlist: MasterPlaylist,
    ) -> Result<(), JsValue> {
        let mut m3u8_url_to_track = HashMap::new();

        let mut track_idx = 0;

        let reference_streams = playlist
            .streams
            .iter()
            .flat_map(|stream| {
                stream
                    .audio
                    .as_ref()
                    .into_iter()
                    .chain(stream.video.as_ref())
                    .map(|group| group.as_str())
            })
            .collect::<HashSet<_>>()
            .into_iter();

        let mut group_to_id = HashMap::new();
        for (group_idx, stream) in reference_streams.enumerate() {
            let Some(groups) = playlist.groups.get_mut(stream) else {
                return Err(JsValue::from_str(&format!("failed to find group for stream: {}", stream)));
            };

            // If we have a default track we need to make sure only 1 track is default
            let pos = groups.iter().position(|item| item.default).unwrap_or(0);
            groups.iter_mut().for_each(|item| item.default = false);
            groups[pos].default = true;

            // Otherwise this is actually a reference track
            // So we need to generate a new track id for it.
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
                            id: track_idx,
                            group_id: group_idx as u32,
                            playlist_url: url.clone(),
                            bandwidth: track.bandwidth,
                            codecs: track.codecs.clone(),
                            frame_rate: track.frame_rate,
                            width: track.resolution.map(|r| r.0),
                            height: track.resolution.map(|r| r.1),
                        };

                        track_idx += 1;

                        t
                    })
                    .id;

                if track.default {
                    self.active_group_track_ids.push(track_id);
                }
            }

            group_to_id.insert(stream, group_idx as u32);
        }

        let mut variants_map = HashMap::new();

        for (id, stream) in playlist.streams.iter().enumerate() {
            let variant = Variant {
                audio_track: stream
                    .audio
                    .as_ref()
                    .and_then(|id| group_to_id.get(id.as_str()).cloned()),
                video_track: stream
                    .video
                    .as_ref()
                    .and_then(|id| group_to_id.get(id.as_str()).cloned()),
                group: stream.group.clone(),
                name: stream.name.clone(),
                bandwidth: stream.bandwidth,
                id: id as u32,
            };

            let codec = format!(
                "video/mp4; codecs=\"{}\"",
                stream
                    .video
                    .iter()
                    .chain(stream.audio.iter())
                    .filter_map(|id| playlist.groups.get(id))
                    .filter_map(|group| group.first())
                    .map(|track| track.codecs.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            );

            if MediaSource::is_type_supported(&codec) {
                variants_map
                    .entry(stream.name.as_str())
                    .or_insert_with(Vec::new)
                    .push(variant);
            }
        }

        let mut scuf_groups = playlist.scuf_groups.iter().collect::<Vec<_>>();
        scuf_groups.sort_by(|(_, a), (_, b)| a.priority.cmp(&b.priority));

        let mut variants = variants_map
            .into_iter()
            .filter_map(|(_, variants)| {
                scuf_groups.iter().find_map(move |(group, _)| {
                    variants.iter().find(|v| v.group == group.as_str()).cloned()
                })
            })
            .collect::<Vec<_>>();

        variants.sort_by(|a, b| b.bandwidth.cmp(&a.bandwidth));

        variants.iter_mut().enumerate().for_each(|(id, variant)| {
            variant.id = id as u32;
        });

        self.variants = variants.clone();

        let mut tracks = m3u8_url_to_track.into_values().collect::<Vec<_>>();
        tracks.sort_by(|a, b| a.id.cmp(&b.id));

        self.track_states = tracks
            .clone()
            .into_iter()
            .map(|t| TrackState::new(t, self.bandwidth.clone()))
            .collect();

        self.inner.set_tracks(tracks, variants, true);
        self.inner
            .set_active_group_track_ids(self.active_group_track_ids.clone());
        self.inner.set_active_variant_id(1);

        Ok(())
    }

    fn handle_media_playlist(
        &mut self,
        input_url: Url,
        playlist: MediaPlaylist,
    ) -> Result<(), JsValue> {
        let track = Track {
            id: 0,
            group_id: 0,
            bandwidth: 0,
            playlist_url: input_url,
            codecs: "".to_string(),
            frame_rate: None,
            height: None,
            width: None,
        };

        let variant = Variant {
            audio_track: Some(0),
            video_track: Some(0),
            group: "default".to_string(),
            name: "default".to_string(),
            bandwidth: 0,
            id: 0,
        };

        let mut track_state = TrackState::new(track.clone(), self.bandwidth.clone());
        track_state.set_playlist(playlist);

        self.track_states = vec![track_state];

        self.inner.set_tracks(vec![track], vec![variant], false);
        self.inner.set_active_variant_id(0);

        Ok(())
    }
}
