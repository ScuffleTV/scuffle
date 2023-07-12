use std::{
    collections::{HashSet, VecDeque},
    io,
};

use bytes::{Buf, Bytes};
use mp4::{
    types::{mdat::Mdat, moof::Moof, moov::Moov},
    DynBox,
};
use serde::Serialize;
use tsify::Tsify;
use url::Url;
use wasm_bindgen::JsValue;

use crate::hls::{
    self,
    media::{MediaPlaylist, RenditionReport},
};

use super::{
    bandwidth::Bandwidth,
    fetch::{FetchRequest, InflightRequest},
    util::now,
};

#[derive(Tsify, Debug, Clone, Serialize)]
#[tsify(into_wasm_abi)]
pub struct Track {
    /// The track id (unique for all tracks)
    pub id: u32,
    /// The group this track belongs to
    pub group_id: u32,
    /// The bandwidth estimate for this track
    pub bandwidth: u32,
    /// The url to the playlist for this track
    pub playlist_url: Url,
    /// The codecs for this track
    pub codecs: String,

    /// The width of this track (if video)
    pub width: Option<u32>,
    /// The height of this track (if video)
    pub height: Option<u32>,
    /// The frame rate of this track (if video)
    pub frame_rate: Option<f64>,
}

#[derive(Tsify, Debug, Clone, Serialize)]
#[tsify(into_wasm_abi)]
pub struct Variant {
    /// The variant id (unique for all variants)
    pub id: u32,
    /// The name of this variant
    pub name: String,
    /// The scuffle group this variant belongs to
    pub group: String,
    /// The group id of the audio track
    pub audio_track: Option<u32>,
    /// The group id of the video track
    pub video_track: Option<u32>,
    /// The bandwidth estimation for this variant
    pub bandwidth: u32,
}

pub struct TrackRequest {
    inflight: Option<InflightRequest>,
    request: FetchRequest,
    is_init: bool,
    is_preload: bool,
}

impl TrackRequest {
    fn new(req: FetchRequest, is_init: bool, is_preload: bool) -> Self {
        Self {
            inflight: None,
            request: req,
            is_init,
            is_preload,
        }
    }
}

pub struct TrackState {
    track: Track,
    playlist_req: Option<InflightRequest>,
    playlist: Option<MediaPlaylist>,

    requests: Requests,

    current_sn: u32,
    current_part: u32,
    current_map_sn: u32,
    was_rendition: bool,

    running: bool,
    low_latency: bool,
    next_playlist_req_time: f64,
    last_fetch_delay: f64,
    last_playlist_fetch: f64,

    preloaded_map: HashSet<Url>,

    last_end_time: f64,

    stop_at: Option<f64>,

    bandwidth: Bandwidth,

    track_info: Option<TrackInfo>,
}

struct Requests {
    max_concurrent_requests: usize,
    inflight: VecDeque<TrackRequest>,
    inflight_urls: HashSet<Url>,
}

impl Requests {
    fn new(max_concurrent_requests: usize) -> Self {
        Self {
            max_concurrent_requests,
            inflight: VecDeque::new(),
            inflight_urls: HashSet::new(),
        }
    }

    fn set_max_concurrent_requests(&mut self, max_concurrent_requests: usize) {
        self.max_concurrent_requests = max_concurrent_requests;
    }

    fn push(&mut self, mut req: TrackRequest) -> Result<(), JsValue> {
        if self.inflight_urls.contains(req.request.url()) {
            return Ok(());
        }

        if self.active_count() < self.max_concurrent_requests && req.inflight.is_none() {
            req.inflight = Some(req.request.start()?);
        }

        self.inflight_urls.insert(req.request.url().clone());
        self.inflight.push_back(req);

        Ok(())
    }

    fn pop(&mut self) -> Result<Option<TrackRequest>, JsValue> {
        let Some(req) = self.inflight.front_mut() else {
            return Ok(None);
        };

        if req.inflight.is_none() {
            req.inflight = Some(req.request.start()?);
        }

        if req.inflight.as_mut().unwrap().is_done() {
            self.inflight_urls.remove(req.request.url());

            let old = self.inflight.pop_front();

            while self.active_count() < self.max_concurrent_requests {
                if let Some(req) = self.inflight.iter_mut().find(|r| r.inflight.is_none()) {
                    req.inflight = Some(req.request.start()?);
                } else {
                    break;
                }
            }

            return Ok(old);
        }

        Ok(None)
    }

    fn active_count(&mut self) -> usize {
        self.inflight
            .iter()
            .filter(|r| {
                r.inflight
                    .as_ref()
                    .map(|i| !i.is_done())
                    .unwrap_or_default()
            })
            .count()
    }

    fn clear(&mut self) {
        self.inflight.clear();
        self.inflight_urls.clear();
    }
}

fn get_url(playlist_url: &str, url: &str) -> Result<Url, JsValue> {
    Url::parse(url).or_else(|_| {
        let playlist_url = Url::parse(playlist_url).map_err(|_| "invalid playlist url")?;
        let url = playlist_url.join(url).map_err(|_| "invalid url")?;

        Ok(url)
    })
}

pub enum TrackResult {
    Init {
        moov: Moov,
    },
    Media {
        fragments: Vec<Fragment>,
        start_time: f64,
        end_time: f64,
    },
}

pub struct Fragment {
    pub moof: Moof,
    pub mdat: Mdat,
    pub start_time: f64,
    pub end_time: f64,
}

struct TrackInfo {
    timescale: u32,
}

fn demux_mp4_boxes(mut cursor: io::Cursor<Bytes>) -> Result<Vec<DynBox>, JsValue> {
    Ok((0..)
        .map_while(|_| {
            if cursor.has_remaining() {
                Some(DynBox::demux(&mut cursor))
            } else {
                None
            }
        })
        .take_while(|r| r.is_ok())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?)
}

impl TrackState {
    pub fn new(track: Track, bandwidth: Bandwidth) -> Self {
        Self {
            track,
            playlist_req: None,
            running: false,
            was_rendition: false,
            playlist: None,
            requests: Requests::new(1),
            current_sn: 0,
            current_map_sn: 0,
            next_playlist_req_time: 0.0,
            last_playlist_fetch: 0.0,
            last_fetch_delay: 0.0,
            low_latency: false,
            current_part: 0,
            last_end_time: 0.0,
            stop_at: None,

            preloaded_map: HashSet::new(),

            bandwidth,

            track_info: None,
        }
    }

    pub fn url(&self) -> &Url {
        &self.track.playlist_url
    }

    pub fn running(&self) -> bool {
        self.running
    }

    pub fn rendition_reports(&self, url: &Url) -> Option<RenditionReport> {
        let playlist = self.playlist.as_ref()?;

        playlist
            .rendition_reports
            .iter()
            .find(|r| {
                let r_url = get_url(self.track.playlist_url.as_str(), &r.uri).unwrap();
                &r_url == url
            })
            .cloned()
    }

    pub fn run(&mut self) -> Result<Option<TrackResult>, JsValue> {
        if !self.running {
            return Ok(None);
        }

        if let Some(mut req) = self.requests.pop()? {
            let inflight = req.inflight.as_mut().unwrap();

            // We have something to yeild to the caller.
            let data = match inflight.result() {
                Ok(Some(data)) => data,
                Ok(None) => {
                    return Err("request should be done".into());
                }
                Err(err) => {
                    if req.is_preload {
                        self.preloaded_map.remove(inflight.url());
                        return Ok(None);
                    } else {
                        return Err(err);
                    }
                }
            };

            self.bandwidth.report_download(&inflight.metrics().unwrap());

            if req.is_init {
                let boxes = demux_mp4_boxes(io::Cursor::new(Bytes::from(data)))?;

                if boxes.is_empty() {
                    return Err("invalid init segment, missing ftyp".into());
                }

                match &boxes[0] {
                    DynBox::Ftyp(ftyp) => ftyp,
                    _ => {
                        return Err("invalid init segment, missing ftyp".into());
                    }
                };

                let moov = boxes
                    .into_iter()
                    .find_map(|b| match b {
                        DynBox::Moov(moov) => Some(moov),
                        _ => None,
                    })
                    .ok_or("invalid init segment, missing moov")?;

                let Some(trak) = moov.traks.get(0) else {
                    return Err("invalid init segment, missing trak".into());
                };

                self.track_info = Some(TrackInfo {
                    timescale: trak.mdia.mdhd.timescale,
                });

                Ok(Some(TrackResult::Init { moov }))
            } else {
                let Some(track_info) = &self.track_info else {
                    return Err("missing track info".into());
                };

                let boxes = demux_mp4_boxes(io::Cursor::new(Bytes::from(data)))?;

                let mut fragments = Vec::new();

                // Convert the boxes vector into a tuple of moof and mdat
                let boxes = boxes
                    .into_iter()
                    .filter_map(|b| match b {
                        DynBox::Moof(moof) => Some((Some(moof), None)),
                        DynBox::Mdat(mdat) => Some((None, Some(mdat))),
                        _ => None,
                    })
                    .try_fold::<_, _, Result<_, &'static str>>(
                        Vec::<(Moof, Option<Mdat>)>::new(),
                        |mut acc, (moof, mdat)| {
                            if let Some(moof) = moof {
                                if let Some(last) = acc.last() {
                                    if last.1.is_none() {
                                        return Err("invalid media segment, missing mdat");
                                    }
                                }

                                acc.push((moof, None));
                            }

                            if let Some(mdat) = mdat {
                                if acc.is_empty() || acc.last().unwrap().1.is_some() {
                                    return Err("invalid media segment, missing moof");
                                }

                                acc.last_mut().unwrap().1 = Some(mdat);
                            }

                            Ok(acc)
                        },
                    )?;

                for (moof, mdat) in boxes {
                    let Some(traf) = moof.traf.get(0) else {
                        return Err("invalid media segment, missing traf".into());
                    };

                    // This will tell us when the fragment starts
                    let base_decode_time = traf
                        .tfdt
                        .as_ref()
                        .map(|tfdt| tfdt.base_media_decode_time)
                        .unwrap_or_default();
                    // This will tell us how long the fragment is (in timescales)
                    let duration = traf.duration() as u64;

                    let start_time = base_decode_time as f64 / track_info.timescale as f64;
                    let end_time =
                        (base_decode_time + duration) as f64 / track_info.timescale as f64;

                    fragments.push(Fragment {
                        moof,
                        mdat: mdat.unwrap(),
                        start_time,
                        end_time,
                    });
                }

                let start_time = fragments.first().map(|f| f.start_time).unwrap_or_default();
                let end_time = fragments.last().map(|f| f.end_time).unwrap_or_default();

                self.last_end_time = end_time;

                if self.stop_at.map(|s| s <= end_time).unwrap_or_default() {
                    self.stop();
                }

                Ok(Some(TrackResult::Media {
                    fragments,
                    start_time,
                    end_time,
                }))
            }
        } else {
            self.request_playlist()?;
            self.handle_playlist()?;

            Ok(None)
        }
    }

    pub fn set_stop_at(&mut self, stop_at: Option<f64>) {
        self.stop_at = stop_at;
        if let Some(stop_at) = self.stop_at {
            if stop_at <= self.last_end_time {
                self.stop();
            }
        }
    }

    pub fn set_low_latency(&mut self, low_latency: bool) {
        self.low_latency = low_latency;
        self.requests
            .set_max_concurrent_requests(if low_latency { 3 } else { 1 })
    }

    pub fn stop_at(&self) -> Option<f64> {
        self.stop_at
    }

    fn handle_playlist(&mut self) -> Result<(), JsValue> {
        let Some(playlist) = &self.playlist else {
            return Ok(());
        };

        if self.current_sn < playlist.media_sequence {
            if let Some(segment) = playlist.segments.iter().rev().find(|s| s.map.is_some()) {
                let url = get_url(
                    self.track.playlist_url.as_str(),
                    segment.map.as_deref().unwrap(),
                )?;
                self.requests.push(TrackRequest::new(
                    FetchRequest::new("GET", url)
                        .header("Accept", "video/mp4")
                        .set_timeout(2000),
                    true,
                    false,
                ))?;
                self.current_map_sn = segment.sn;
            }

            self.current_sn = if self.low_latency {
                let last_sn = playlist
                    .segments
                    .last()
                    .map(|s| s.sn)
                    .unwrap_or(playlist.media_sequence);
                last_sn
            } else {
                let segments = playlist
                    .segments
                    .iter()
                    .rev()
                    .filter_map(|s| if s.url.is_empty() { None } else { Some(s.sn) })
                    .collect::<Vec<_>>();
                match segments.len() {
                    0 => playlist.media_sequence,
                    1 => segments[0],
                    _ => segments[1],
                }
            };
            self.current_part = 0;
        }

        for segment in &playlist.segments {
            if segment.sn < self.current_sn {
                continue;
            }

            if let Some(map) = &segment.map {
                if self.current_map_sn < segment.sn {
                    let url = get_url(self.track.playlist_url.as_str(), map)?;
                    self.requests.push(TrackRequest::new(
                        FetchRequest::new("GET", url)
                            .header("Accept", "video/mp4")
                            .set_timeout(2000),
                        true,
                        false,
                    ))?;
                }
            } else if self.current_map_sn < segment.sn.checked_sub(1).unwrap_or_default() {
                // Since its now possible to start at any segment, we need to make sure we have
                // the map for the previous segment
                let url = get_url(
                    self.track.playlist_url.as_str(),
                    playlist
                        .segments
                        .iter()
                        .rev()
                        .find_map(|s| s.map.as_ref())
                        .ok_or("missing map")?,
                )?;
                self.requests.push(TrackRequest::new(
                    FetchRequest::new("GET", url)
                        .header("Accept", "video/mp4")
                        .set_timeout(2000),
                    true,
                    false,
                ))?;
            }

            self.current_map_sn = segment.sn;

            if self.low_latency {
                if segment.parts.is_empty() && segment.url.is_empty() {
                    break;
                }

                if !segment.url.is_empty() && self.current_part == 0 {
                    // We havent loaded this segment yet and it has a completed url
                    // So we just request that instead
                    let url = get_url(self.track.playlist_url.as_str(), &segment.url)?;
                    self.last_fetch_delay = segment.duration / 2.0 * 1000.0;
                    self.requests.push(TrackRequest::new(
                        FetchRequest::new("GET", url)
                            .header("Accept", "video/mp4")
                            .set_timeout((segment.duration * 1000.0) as u32 + 1500),
                        false,
                        false,
                    ))?;
                } else {
                    for part in segment.parts.iter().skip(self.current_part as usize) {
                        let url = get_url(self.track.playlist_url.as_str(), &part.uri)?;
                        if !self.preloaded_map.remove(&url) {
                            self.last_fetch_delay = part.duration * 1000.0 / 2.0;
                            self.requests.push(TrackRequest::new(
                                FetchRequest::new("GET", url)
                                    .header("Accept", "video/mp4")
                                    .set_timeout((part.duration * 1000.0) as u32 + 1500),
                                false,
                                false,
                            ))?;
                        }

                        self.current_part += 1;
                    }
                }

                if !segment.url.is_empty() {
                    self.current_sn = segment.sn + 1;
                    self.current_part = 0;
                }
            } else {
                if segment.url.is_empty() {
                    break;
                }

                self.current_sn = segment.sn + 1;
                self.current_part = 0;
                let url = get_url(self.track.playlist_url.as_str(), &segment.url)?;
                self.last_fetch_delay = segment.duration / 2.0 * 1000.0;
                self.requests.push(TrackRequest::new(
                    FetchRequest::new("GET", url)
                        .header("Accept", "video/mp4")
                        .set_timeout((segment.duration * 1000.0) as u32 + 1500),
                    false,
                    false,
                ))?;
            }
        }

        if playlist.end_list {
            self.next_playlist_req_time = -1.0;
        } else if self.next_playlist_req_time == -1.0 {
            self.next_playlist_req_time = now() + self.last_fetch_delay;
        }

        let msn = playlist
            .segments
            .last()
            .map(|s| s.sn)
            .unwrap_or(playlist.media_sequence);
        if msn < self.current_sn {
            return Ok(());
        }

        if self.low_latency {
            for hint in &playlist.preload_hint {
                let is_init = hint.hint_type.to_uppercase() == "MAP";
                let url = get_url(self.track.playlist_url.as_str(), &hint.uri)?;
                if self.preloaded_map.insert(url.clone()) {
                    self.requests.push(TrackRequest::new(
                        FetchRequest::new("GET", url)
                            .header("Accept", "video/mp4")
                            .set_timeout(
                                (playlist.part_target_duration.unwrap_or(0.5) * 1000.0) as u32
                                    + 1500,
                            ),
                        is_init,
                        true,
                    ))?;
                }
            }
        }

        Ok(())
    }

    fn request_playlist(&mut self) -> Result<(), JsValue> {
        if let Some(req) = self.playlist_req.as_mut() {
            if let Some(result) = req.result()? {
                match hls::Playlist::try_from(result.as_slice()) {
                    Ok(hls::Playlist::Media(playlist)) => {
                        self.playlist = Some(playlist);
                        self.next_playlist_req_time = -1.0;
                        self.last_playlist_fetch = now();
                    }
                    Err(err) => {
                        tracing::error!("failed to parse playlist: {}", err);
                        self.next_playlist_req_time = now();
                    }
                    _ => {
                        return Err("invalid playlist".into());
                    }
                }

                self.playlist_req = None;
            }
        }

        if self.next_playlist_req_time != -1.0 && self.playlist_req.is_none() {
            if self.low_latency
                && (self
                    .playlist
                    .as_ref()
                    .and_then(|p| p.server_control.as_ref().map(|s| s.can_block_reload))
                    .unwrap_or_default()
                    || self.was_rendition)
            {
                // Low latency request
                let mut url = self.track.playlist_url.clone();

                self.was_rendition = false;

                url.query_pairs_mut()
                    .append_pair("_HLS_msn", self.current_sn.to_string().as_str());
                url.query_pairs_mut()
                    .append_pair("_HLS_part", self.current_part.to_string().as_str());

                self.playlist_req = Some(
                    FetchRequest::new("GET", url)
                        .header("Accept", "application/vnd.apple.mpegurl")
                        .set_timeout(5000)
                        .start()?,
                );
            } else if now() >= self.next_playlist_req_time {
                self.playlist_req = Some(
                    FetchRequest::new("GET", self.track.playlist_url.clone())
                        .header("Accept", "application/vnd.apple.mpegurl")
                        .set_timeout(2000)
                        .start()?,
                );
            }
        }

        Ok(())
    }

    pub fn start(&mut self) {
        self.running = true;
        self.stop_at = None;
        self.was_rendition = false;
    }

    pub fn start_at(&mut self, current_sn: u32) {
        self.running = true;
        self.current_sn = current_sn;
        self.current_part = 0;
        self.was_rendition = true;
        self.stop_at = None;
    }

    pub fn stop(&mut self) {
        self.running = false;
        self.playlist = None;
        self.playlist_req = None;
        self.stop_at = None;
        self.current_part = 0;
        self.current_sn = 0;
        self.last_end_time = 0.0;
        self.last_fetch_delay = 0.0;
        self.last_playlist_fetch = 0.0;
        self.next_playlist_req_time = 0.0;
        self.requests.clear();
        self.preloaded_map.clear();
    }

    pub fn set_playlist(&mut self, playlist: MediaPlaylist) {
        self.playlist = Some(playlist);
    }
}
