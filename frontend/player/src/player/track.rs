use std::{collections::VecDeque, io};

use bytes::{Buf, Bytes};
use mp4::{
    types::{mdat::Mdat, moof::Moof, moov::Moov},
    DynBox,
};
use serde::Serialize;
use tsify::Tsify;
use url::Url;
use wasm_bindgen::JsValue;
use web_sys::window;

use crate::hls::{self, media::MediaPlaylist};

use super::fetch::{FetchRequest, InflightRequest};

#[derive(Tsify, Debug, Clone, Serialize)]
#[tsify(into_wasm_abi)]
pub struct Track {
    pub id: u32,
    pub bandwidth: Option<u32>,
    pub name: Option<String>,
    pub playlist_url: Url,
    pub referenced_group_ids: Vec<u32>,
    pub is_variant_track: bool,
    pub codecs: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub frame_rate: Option<f64>,
    pub reference: Option<ReferenceTrack>,
}

#[derive(Tsify, Debug, Clone, Serialize)]
#[tsify(into_wasm_abi)]
pub struct ReferenceTrack {
    pub group_id: u32,
    pub is_default: bool,
}

pub struct TrackRequest {
    req: InflightRequest,
    is_init: bool,
}

pub struct TrackState {
    track: Track,
    playlist_req: Option<InflightRequest>,
    playlist: Option<MediaPlaylist>,

    requests: VecDeque<TrackRequest>,

    current_sn: u32,
    current_part: u32,
    current_map_sn: u32,

    running: bool,
    low_latency: bool,
    next_playlist_req_time: f64,
    last_fetch_delay: f64,
    last_playlist_fetch: f64,

    last_end_time: f64,

    stop_at: Option<f64>,

    track_info: Option<TrackInfo>,
}

fn now() -> f64 {
    window().unwrap().performance().unwrap().now()
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
        .map_err(|_| "invalid init segment")?)
}

impl TrackState {
    pub fn new(track: Track) -> Self {
        Self {
            track,
            playlist_req: None,
            running: false,
            playlist: None,
            requests: VecDeque::new(),
            current_sn: 0,
            current_map_sn: 0,
            next_playlist_req_time: 0.0,
            last_playlist_fetch: 0.0,
            last_fetch_delay: 0.0,
            low_latency: false,
            current_part: 0,
            last_end_time: 0.0,
            stop_at: None,

            track_info: None,
        }
    }

    pub fn running(&self) -> bool {
        self.running
    }

    pub fn run(&mut self) -> Result<Option<TrackResult>, JsValue> {
        if !self.running {
            return Ok(None);
        }

        if let Some(req) = self.handle_requests()? {
            // We have something to yeild to the caller.
            let data = req.req.result()?.expect("request should be done");
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

                let mut keyframe = 0;

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

                    if traf.contains_keyframe() {
                        keyframe += 1;
                    }

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

                if let Some(stop_at) = self.stop_at {
                    if end_time >= stop_at && keyframe > 0 {
                        self.stop();
                    }
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

    fn handle_requests(&mut self) -> Result<Option<TrackRequest>, JsValue> {
        let Some(req) = self.requests.front() else {
            return Ok(None);
        };

        if req.req.is_done() {
            Ok(self.requests.pop_front())
        } else {
            Ok(None)
        }
    }

    pub fn set_stop_at(&mut self, stop_at: Option<f64>) {
        self.stop_at = stop_at;
    }

    pub fn set_low_latency(&mut self, low_latency: bool) {
        self.low_latency = low_latency;
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
                self.requests.push_back(TrackRequest {
                    req: FetchRequest::new("GET", url.as_str())
                        .header("Accept", "video/mp4")
                        .set_timeout(2000)
                        .start()?,
                    is_init: true,
                });
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
                    self.requests.push_back(TrackRequest {
                        req: FetchRequest::new("GET", url.as_str())
                            .header("Accept", "video/mp4")
                            .set_timeout(2000)
                            .start()?,
                        is_init: true,
                    });
                }
            }

            self.current_map_sn = segment.sn;

            if segment.url.is_empty() && (!self.low_latency || segment.parts.is_empty()) {
                continue;
            }

            let url = if self.low_latency && segment.parts.len() > self.current_part as usize {
                let part = &segment.parts[self.current_part as usize];
                self.current_part += 1;
                self.last_fetch_delay = part.duration * 1000.0 / 2.0;
                part.uri.as_str()
            } else if !segment.url.is_empty() {
                self.current_part = 0;
                self.current_sn = segment.sn + 1;
                self.last_fetch_delay = segment.duration / 2.0 * 1000.0;
                segment.url.as_str()
            } else {
                continue;
            };

            let url = get_url(self.track.playlist_url.as_str(), url)?;
            self.requests.push_back(TrackRequest {
                req: FetchRequest::new("GET", url.as_str())
                    .header("Accept", "video/mp4")
                    .set_timeout(2000)
                    .start()?,
                is_init: false,
            });

            if self.low_latency
                && segment.parts.len() == self.current_part as usize
                && !segment.url.is_empty()
            {
                // We are finished with this segment
                self.current_sn = segment.sn + 1;
                self.current_part = 0;
                continue;
            }
        }

        // If the playlist has an end list tag we don't need to request it again
        if playlist.end_list {
            self.next_playlist_req_time = -1.0;
        } else if self.next_playlist_req_time == -1.0 {
            self.next_playlist_req_time = now() + self.last_fetch_delay;
        }

        Ok(())
    }

    fn request_playlist(&mut self) -> Result<(), JsValue> {
        if let Some(req) = self.playlist_req.as_ref() {
            if let Some(result) = req.result()? {
                match hls::Playlist::try_from(result.as_slice())? {
                    hls::Playlist::Media(playlist) => {
                        self.playlist = Some(playlist);
                    }
                    _ => {
                        return Err("invalid playlist".into());
                    }
                }

                self.playlist_req = None;
                self.next_playlist_req_time = -1.0;
                self.last_playlist_fetch = now();
            }
        } else if self.next_playlist_req_time != -1.0 {
            if self.low_latency
                && self
                    .playlist
                    .as_ref()
                    .and_then(|p| p.server_control.as_ref().map(|s| s.can_block_reload))
                    .unwrap_or_default()
            {
                // Low latency request
                let mut url = self.track.playlist_url.clone();

                url.query_pairs_mut()
                    .append_pair("_HLS_msn", self.current_sn.to_string().as_str());
                url.query_pairs_mut()
                    .append_pair("_HLS_part", self.current_part.to_string().as_str());

                self.playlist_req = Some(
                    FetchRequest::new("GET", url.as_str())
                        .header("Accept", "application/vnd.apple.mpegurl")
                        .set_timeout(2000)
                        .start()?,
                );
            } else if now() >= self.next_playlist_req_time {
                self.playlist_req = Some(
                    FetchRequest::new("GET", self.track.playlist_url.as_str())
                        .header("Accept", "application/vnd.apple.mpegurl")
                        .set_timeout(2000)
                        .start()?,
                );
            }
        }

        Ok(())
    }

    pub fn track(&self) -> &Track {
        &self.track
    }

    pub fn start(&mut self) {
        self.running = true;
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
    }

    pub fn set_playlist(&mut self, playlist: MediaPlaylist) {
        self.playlist = Some(playlist);
    }
}
