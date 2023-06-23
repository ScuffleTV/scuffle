use crate::global::GlobalState;
use anyhow::{anyhow, Context, Result};
use bytes::Bytes;
use bytesio::bytes_writer::BytesWriter;
use chrono::SecondsFormat;
use common::prelude::FutureTimeout;
use fred::{
    prelude::{HashesInterface, KeysInterface},
    types::{Expiration, RedisValue},
};
use futures_util::StreamExt;
use mp4::{
    types::{
        ftyp::{FourCC, Ftyp},
        mdat::Mdat,
        mfhd::Mfhd,
        moof::Moof,
        moov::Moov,
        mvex::Mvex,
        mvhd::Mvhd,
        tfdt::Tfdt,
        tfhd::Tfhd,
        traf::Traf,
        trex::Trex,
        trun::Trun,
    },
    BoxType,
};
use std::{collections::HashMap, io, pin::pin, sync::Arc, time::Duration};
use tokio::{net::UnixListener, select, sync::mpsc};
use tokio_util::sync::CancellationToken;

use super::{
    renditions::RenditionMap,
    track_parser::{track_parser, TrackOut, TrackSample},
    utils::{release_lock, set_lock, unix_stream},
};

mod consts;
pub(crate) mod state;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operation {
    Init,
    Fragments,
}

#[derive(Default, Clone)]
struct TrackState {
    moov: Option<Moov>,
    timescale: u32,
    samples: Vec<(usize, TrackSample)>,
}

struct Variant {
    stream_id: String,
    variant_id: String,
    request_id: String,
    operation: Operation,
    lock_owner: CancellationToken,
    tracks: Vec<TrackState>,
    redis_state: state::PlaylistState,
    should_discontinuity: bool,
    segment_state: HashMap<u32, (state::SegmentState, HashMap<u32, Bytes>)>,
    ready: mpsc::Sender<()>,
    is_ready: bool,
    renditions: Arc<RenditionMap>,
}

pub async fn handle_variant(
    global: Arc<GlobalState>,
    ready: mpsc::Sender<()>,
    stream_id: String,
    variant_id: String,
    request_id: String,
    track: UnixListener,
    renditions: Arc<RenditionMap>,
) -> Result<String, ()> {
    let mut variant = Variant::new(ready, 1, stream_id, variant_id, request_id, renditions);

    variant
        .run(
            global,
            pin!(track_parser(pin!(unix_stream(track, 256 * 1024))).map(|r| (r, 1))),
        )
        .await?;

    Ok(variant.variant_id)
}

impl Variant {
    pub fn new(
        ready: mpsc::Sender<()>,
        trak_count: u32,
        stream_id: String,
        variant_id: String,
        request_id: String,
        renditions: Arc<RenditionMap>,
    ) -> Self {
        Self {
            stream_id,
            variant_id,
            request_id,
            tracks: vec![TrackState::default(); trak_count as usize],
            operation: Operation::Init,
            lock_owner: CancellationToken::new(),
            redis_state: state::PlaylistState::default(),
            segment_state: HashMap::new(),
            should_discontinuity: false,
            ready,
            renditions,
            is_ready: false,
        }
    }

    #[tracing::instrument(skip(self, global, tracks), fields(stream_id = %self.stream_id, variant_id = %self.variant_id, request_id = %self.request_id))]
    pub async fn run(
        &mut self,
        global: Arc<GlobalState>,
        tracks: impl futures::Stream<Item = (io::Result<TrackOut>, u32)> + Unpin,
    ) -> Result<(), ()> {
        let mut set_lock_fut = pin!(set_lock(
            global.clone(),
            consts::redis_mutex_key(&self.stream_id.to_string(), &self.variant_id.to_string()),
            self.request_id.clone(),
            self.lock_owner.clone(),
        ));

        let mut tracks = tracks.enumerate();

        let mut result = Ok(());

        loop {
            select! {
                item = tracks.next() => {
                    match item {
                        Some((_, (Ok(TrackOut::Moov(moov)), track_id))) => {
                            let idx = track_id as usize - 1;

                            if self.tracks.len() <= idx {
                                tracing::error!("track {} unexpected but moov received", track_id);
                                result = Err(());
                                break;
                            }

                            self.tracks[idx].moov = Some(moov);
                        }
                        Some((stream_idx, (Ok(TrackOut::Sample(sample)), track_id))) => {
                            let idx = track_id as usize - 1;

                            if self.tracks.len() <= idx {
                                tracing::error!("track {:#} unexpected but moov received", track_id);
                                result = Err(());
                                break;
                            }

                            self.tracks[idx].samples.push((stream_idx, sample));
                        }
                        Some((_, (Err(err), idx))) => {
                            tracing::error!("track {} error: {:#}", idx, err);
                            result = Err(());
                            break;
                        }
                        None => {
                            tracing::debug!("tracks closed");
                            break;
                        }
                    }
                }
                r = &mut set_lock_fut => {
                    if let Err(err) = r {
                        tracing::error!("set lock error: {:#}", err);
                    } else {
                        tracing::warn!("set lock done prematurely without error");
                    }

                    break;
                }
            }

            select! {
                r = self.process(&global) => {
                    if let Err(err) = r {
                        tracing::error!("process error: {:#}", err);
                        result = Err(());
                        break;
                    }
                }
                r = &mut set_lock_fut => {
                    if let Err(err) = r {
                        tracing::error!("set lock error: {:#}", err);
                    } else {
                        tracing::warn!("set lock done prematurely without error");
                    }

                    break;
                }
            }
        }

        tracing::debug!("track closed");

        if let Err(err) = self.handle_shutdown(&global).await {
            tracing::error!("handle shutdown error: {:#}", err);
        }

        tracing::debug!("track flushed");

        if let Err(err) = release_lock(
            &global,
            &consts::redis_mutex_key(&self.stream_id, &self.variant_id),
            &self.request_id,
        )
        .await
        {
            tracing::error!("release lock error: {:#}", err);
        }

        tracing::debug!("track complete");

        result
    }

    async fn handle_shutdown(&mut self, global: &Arc<GlobalState>) -> Result<()> {
        if self.operation == Operation::Init {
            return Ok(());
        }

        let samples = self
            .tracks
            .iter_mut()
            .map(|track| track.samples.drain(..).map(|s| s.1).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        if samples.iter().any(|s| !s.is_empty()) {
            tracing::info!(
                "flushing remaining samples {:?}",
                samples.iter().map(|s| s.len()).collect::<Vec<_>>()
            );
            if let std::collections::hash_map::Entry::Vacant(e) = self
                .segment_state
                .entry(self.redis_state.current_segment_idx())
            {
                // This really sucks because we have to create an entire new segment for these few ending samples
                self.redis_state.set_current_fragment_idx(0);
                e.insert(Default::default());
            }

            self.create_fragment(samples)?;
        }

        if self
            .segment_state
            .contains_key(&self.redis_state.current_segment_idx())
        {
            self.segment_state
                .get_mut(&self.redis_state.current_segment_idx())
                .unwrap()
                .0
                .set_ready(true);
            self.redis_state.set_current_fragment_idx(0);
            self.redis_state
                .set_current_segment_idx(self.redis_state.current_segment_idx() + 1);
        }

        let pipeline = global.redis.pipeline();
        if self.update_keys(&pipeline).await? {
            self.refresh_keys(&pipeline).await?;
            pipeline.all().await?;
        }

        Ok(())
    }

    async fn process(&mut self, global: &Arc<GlobalState>) -> Result<()> {
        match self.operation {
            Operation::Init => {
                self.construct_init(global).await?;
            }
            Operation::Fragments => {
                self.handle_sample(global).await?;
                self.update_renditions();

                let pipeline = global.redis.pipeline();
                if self.update_keys(&pipeline).await? {
                    self.refresh_keys(&pipeline).await?;
                    pipeline.all().await?;

                    if !self.is_ready {
                        self.is_ready = true;
                        self.ready.send(()).await?;
                    }
                }
            }
        }

        Ok(())
    }

    fn update_renditions(&self) {
        let current_segment_idx = self.redis_state.current_segment_idx()
            - if self.redis_state.current_fragment_idx() == 0
                && self.redis_state.current_segment_idx() != 0
            {
                1
            } else {
                0
            };
        let current_fragment_idx = self
            .segment_state
            .get(&current_segment_idx)
            .map(|(segment, _)| segment.fragments().len())
            .unwrap_or(0) as u32;
        let current_fragment_idx = if current_fragment_idx != 0 {
            current_fragment_idx - 1
        } else {
            0
        };

        self.renditions
            .set(&self.variant_id, current_segment_idx, current_fragment_idx);
    }

    async fn construct_init(&mut self, global: &Arc<GlobalState>) -> Result<()> {
        if self.tracks.iter().any(|track| track.moov.is_none()) {
            return Ok(());
        }

        let (traks, trexs) = self
            .tracks
            .iter_mut()
            .enumerate()
            .map(|(idx, track)| {
                let track_id = idx as u32 + 1;
                let mut moov = track.moov.take().unwrap();

                if moov.traks.len() != 1 {
                    return Err(anyhow!("expected 1 trak"));
                }

                let mut trak = moov.traks.remove(0);

                trak.edts = None;
                trak.tkhd.track_id = track_id;

                track.timescale = trak.mdia.mdhd.timescale;

                Ok((trak, Trex::new(track_id)))
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .unzip::<_, _, Vec<_>, Vec<_>>();

        let ftyp = Ftyp::new(
            FourCC::Iso5,
            512,
            vec![FourCC::Iso5, FourCC::Iso6, FourCC::Mp41],
        );
        let moov = Moov::new(
            Mvhd::new(0, 0, 1000, 0, 2),
            traks,
            Some(Mvex::new(trexs, None)),
        );

        let mut writer = BytesWriter::default();
        ftyp.mux(&mut writer)?;
        moov.mux(&mut writer)?;

        self.initial_redis_state(global, writer.dispose()).await?;

        self.operation = Operation::Fragments;

        Ok(())
    }

    async fn initial_redis_state(
        &mut self,
        global: &Arc<GlobalState>,
        init_segment: Bytes,
    ) -> Result<()> {
        // At this point we know enough about the stream to look at redis to see if we are resuming, or starting fresh.
        // If we are resuming we need to load some state about what we have already sent to the client.
        // If we are starting fresh we need to create some state so that we can resume later (if needed).
        // We also need to make sure that the previous instance is finished, if not we need to wait for it to finish.
        if self
            .lock_owner
            .cancelled()
            .timeout(Duration::from_secs(5))
            .await
            .is_err()
        {
            return Err(anyhow!("timeout waiting for lock"));
        }

        // We now are the proud owner of the stream and can do whatever we want with it!
        // Get the redis state for the stream.
        let state: HashMap<String, String> = global
            .redis
            .hgetall(consts::redis_state_key(&self.stream_id, &self.variant_id))
            .await
            .context("failed to get redis state")?;

        if !state.is_empty() {
            // We need to validate the state we got from redis.
            let state = state::PlaylistState::from(state);
            if self.tracks.len() != state.track_count() {
                return Err(anyhow!("track count mismatch"));
            }

            for (idx, track) in self.tracks.iter().enumerate() {
                if track.timescale != state.track_timescale(idx).unwrap_or(0) {
                    return Err(anyhow!("track {} timescale mismatch", idx));
                }
            }

            self.redis_state = state;
        } else {
            self.redis_state = state::PlaylistState::default();
            self.tracks.iter().for_each(|track| {
                self.redis_state.insert_track(state::Track {
                    duration: 0,
                    timescale: track.timescale,
                });
            });
        }

        // Since we now know the redis_state we can see if we are starting fresh or resuming.
        if self.redis_state.current_segment_idx() != 0
            || self.redis_state.current_fragment_idx() != 0
        {
            // We now need to fetch the segments from redis.
            let start_idx = (self.redis_state.current_segment_idx() as i32 - 4).max(0) as u32;
            let end_idx = self.redis_state.current_segment_idx()
                + if self.redis_state.current_fragment_idx() == 0 {
                    0
                } else {
                    1
                };
            for idx in start_idx..end_idx {
                let segment: HashMap<String, String> = global
                    .redis
                    .hgetall(consts::redis_segment_state_key(
                        &self.stream_id,
                        &self.variant_id,
                        idx,
                    ))
                    .await
                    .context("failed to get redis segment state")?;
                let segment = state::SegmentState::from(segment);
                self.segment_state.insert(idx, (segment, HashMap::new()));
            }
        }

        let pipeline = global.redis.pipeline();

        let _: RedisValue = pipeline
            .set(
                consts::redis_init_key(&self.stream_id, &self.variant_id),
                init_segment,
                Some(Expiration::EX(consts::ACTIVE_EXPIRE_SECONDS)),
                None,
                false,
            )
            .await
            .context("failed to set redis init")?;

        self.update_keys(&pipeline)
            .await
            .context("failed to update redis keys")?;

        self.refresh_keys(&pipeline)
            .await
            .context("failed to refresh redis keys")?;

        let _: Vec<()> = pipeline
            .all()
            .await
            .context("failed to execute redis pipeline")?;

        self.should_discontinuity = self.redis_state.current_fragment_idx() != 0;

        Ok(())
    }

    async fn handle_sample(&mut self, _global: &Arc<GlobalState>) -> Result<()> {
        // We need to check if we have enough samples to create a fragment.
        // And then check if we do, if we have enough fragments to create a segment.
        if self.should_discontinuity {
            // Discontinuities are a bit of a special case.
            // Any samples before the keyframe on track 1 are discarded.
            // Samples on other tracks will be added to the next fragment.
            let Some(idx) = self.tracks[0]
                .samples
                .iter()
                .position(|(_, sample)| sample.keyframe)
            else {
                // We dont have a place to create a discontinuity yet, so we need to wait a little bit.
                return Ok(());
            };

            // We need to discard all samples on track[0] before the keyframe. (there shouldnt be any, but just in case)
            self.tracks[0].samples.drain(..idx);

            self.redis_state.set_current_fragment_idx(0);
            let current_idx = self.redis_state.current_segment_idx();

            self.redis_state.set_current_segment_idx(current_idx + 1);
            self.redis_state
                .set_discontinuity_sequence(self.redis_state.discontinuity_sequence() + 1);

            // make sure the previous segment is marked as ready.
            if let Some((previous, _)) = self.segment_state.get_mut(&current_idx) {
                previous.set_ready(true);
            }

            self.should_discontinuity = false;
            let mut segment = state::SegmentState::default();
            segment.set_discontinuity(true);
            self.segment_state.insert(
                self.redis_state.current_segment_idx(),
                (segment, HashMap::new()),
            );
        }

        // We need to check if we have enough samples to create a new fragment
        // We only care about the duration from track 1. Other tracks will just be added regardless if they cut or not.

        // We need to also know about the current segment duration incase we can cut a new segment.
        let total_segment_timescale_duration = self
            .segment_state
            .entry(self.redis_state.current_segment_idx())
            .or_insert_with(Default::default)
            .0
            .fragments()
            .iter()
            .map(|fragment| fragment.duration)
            .sum::<u32>();

        // We need to see if the next samples can create a segment.
        // To do this we need to iterate over the samples to find out if any of them are keyframes.
        // If we have a keyframe we need to check if the samples before the keyframe can create a fragment.
        let (sample_durations, segment_durations) = {
            let mut total_sample_timescale_duration = 0;
            self.tracks[0]
                .samples
                .iter()
                .map(|(_, sample)| {
                    total_sample_timescale_duration += sample.duration;
                    (
                        total_sample_timescale_duration as f64 / self.tracks[0].timescale as f64,
                        // We only calculate the segment duration if we have a keyframe.
                        // Since we cant cut a segment without a keyframe.
                        if sample.keyframe {
                            (total_sample_timescale_duration + total_segment_timescale_duration
                                - sample.duration) as f64
                                / self.tracks[0].timescale as f64
                        } else {
                            0.0
                        },
                    )
                })
                .unzip::<_, _, Vec<_>, Vec<_>>() // unzip the tuples into two vectors.
        };

        let idx = segment_durations
            .iter()
            .position(|duration| *duration >= consts::SEGMENT_CUT_TARGET_DURATION);

        // If we have an index we can cut a new segment.
        if let Some(idx) = idx {
            let Some((segment, _)) = self
                .segment_state
                .get_mut(&self.redis_state.current_segment_idx())
            else {
                // This should never happen, but just in case.
                return Err(anyhow!("failed to get current segment state"));
            };

            let last_sample_stream_idx = self.tracks[0].samples[idx].0;
            let samples = self
                .tracks
                .iter_mut()
                .map(|track| {
                    let upper_bound = track
                        .samples
                        .iter()
                        .enumerate()
                        .find_map(|(idx, (stream_idx, _))| {
                            if *stream_idx >= last_sample_stream_idx {
                                // We dont add 1 here because we want to include the last sample.
                                return Some(idx);
                            }

                            None
                        })
                        .unwrap_or_default();

                    track
                        .samples
                        .drain(..upper_bound)
                        .map(|(_, sample)| sample)
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();

            segment.set_ready(true);

            if !samples[0].is_empty() {
                self.create_fragment(samples)?;
            }

            self.redis_state
                .set_current_segment_idx(self.redis_state.current_segment_idx() + 1);
            self.redis_state.set_current_fragment_idx(0);

            return Ok(());
        }

        // We want to find out if we have enough samples to create a fragment.
        let Some(idx) = sample_durations
            .iter()
            .enumerate()
            .find_map(|(idx, duration)| {
                if *duration >= consts::FRAGMENT_CUT_TARGET_DURATION
                    && (*duration * 1000.0).fract() == 0.0
                {
                    return Some(Some(idx));
                }

                if *duration >= consts::FRAGMENT_CUT_MAX_DURATION {
                    return Some(None);
                }

                None
            })
        else {
            // We dont have a place to create a fragment yet, so we need to wait a little bit.
            return Ok(());
        };

        let Some(idx) = idx.or_else(|| {
            sample_durations
                .iter()
                .position(|d| *d >= consts::FRAGMENT_CUT_TARGET_DURATION)
        }) else {
            // We dont have a place to create a fragment yet, so we need to wait a little bit.
            return Ok(());
        };

        let last_sample_stream_idx = self.tracks[0].samples[idx].0;

        // We now extract all the samples we need to create the next fragment.
        let samples = self
            .tracks
            .iter_mut()
            .map(|track| {
                let upper_bound = track
                    .samples
                    .iter()
                    .position(|(stream_idx, _)| *stream_idx >= last_sample_stream_idx)
                    .map(|idx| idx + 1)
                    .unwrap_or_else(|| track.samples.len());

                track
                    .samples
                    .drain(..upper_bound)
                    .map(|(_, sample)| sample)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        self.create_fragment(samples)?;

        Ok(())
    }

    fn create_fragment(&mut self, samples: Vec<Vec<TrackSample>>) -> Result<()> {
        // Get the current segment
        let Some((segment, segment_data_state)) = self
            .segment_state
            .get_mut(&self.redis_state.current_segment_idx())
        else {
            return Err(anyhow!("failed to get current segment"));
        };

        let contains_keyframe = samples[0].iter().any(|sample| sample.keyframe);
        segment.insert_fragment(state::Fragment {
            duration: samples[0].iter().map(|sample| sample.duration).sum(),
            keyframe: contains_keyframe,
        });

        let mut moof = Moof::new(
            Mfhd::new(self.redis_state.sequence_number()),
            samples
                .iter()
                .enumerate()
                .map(|(idx, samples)| {
                    let mut traf = Traf::new(
                        Tfhd::new(idx as u32 + 1, None, None, None, None, None),
                        Some(Trun::new(
                            samples.iter().map(|s| s.sample.clone()).collect(),
                            None,
                        )),
                        Some(Tfdt::new(self.redis_state.track_duration(idx).unwrap())),
                    );

                    traf.optimize();

                    traf
                })
                .collect(),
        );

        let moof_size = moof.size();

        let track_sizes = samples
            .iter()
            .map(|s| s.iter().map(|s| s.data.len()).sum::<usize>())
            .collect::<Vec<_>>();

        moof.traf.iter_mut().enumerate().for_each(|(idx, traf)| {
            let trun = traf.trun.as_mut().unwrap();

            // The base is moof, so we offset by the moof, then we offset by the size of all the previous tracks + 8 bytes for the mdat header.
            trun.data_offset =
                Some(moof_size as i32 + track_sizes[..idx].iter().sum::<usize>() as i32 + 8);
        });

        let mdat = Mdat::new(
            samples
                .iter()
                .flat_map(|s| s.iter().map(|s| s.data.clone()))
                .collect(),
        );

        let mut writer = BytesWriter::default();
        moof.mux(&mut writer)?;
        mdat.mux(&mut writer)?;

        segment_data_state.insert(self.redis_state.current_fragment_idx(), writer.dispose());

        self.redis_state
            .set_sequence_number(self.redis_state.sequence_number() + 1);
        self.redis_state
            .set_current_fragment_idx(self.redis_state.current_fragment_idx() + 1);
        samples.iter().enumerate().for_each(|(idx, s)| {
            self.redis_state.set_track_duration(
                idx,
                self.redis_state.track_duration(idx).unwrap()
                    + s.iter().map(|s| s.duration).sum::<u32>() as u64,
            );
        });

        Ok(())
    }

    async fn update_keys<R: KeysInterface + HashesInterface + Sync>(
        &mut self,
        redis: &R,
    ) -> Result<bool> {
        self.generate_playlist()
            .context("failed to generate playlist")?;

        let mut updated = false;
        {
            let mutations = self.redis_state.extract_mutations();
            if !mutations.is_empty() {
                updated = true;
                let _: RedisValue = redis
                    .hmset(
                        consts::redis_state_key(&self.stream_id, &self.variant_id),
                        mutations,
                    )
                    .await
                    .context("failed to set redis state")?;
            }
        }

        for (idx, (segment, segment_data_state)) in self.segment_state.iter_mut() {
            let mutations = segment.extract_mutations();
            if !mutations.is_empty() {
                updated = true;
                let _: RedisValue = redis
                    .hmset(
                        consts::redis_segment_state_key(&self.stream_id, &self.variant_id, *idx),
                        mutations,
                    )
                    .await
                    .context("failed to set redis segment state")?;
            }

            let data_state_mutations = std::mem::take(segment_data_state);
            if !data_state_mutations.is_empty() {
                updated = true;
                let _: RedisValue = redis
                    .hmset(
                        consts::redis_segment_data_key(&self.stream_id, &self.variant_id, *idx),
                        data_state_mutations,
                    )
                    .await
                    .context("failed to set redis segment data state")?;
            }
        }

        Ok(updated)
    }

    async fn refresh_keys<R: KeysInterface + HashesInterface + Sync>(
        &mut self,
        redis: &R,
    ) -> Result<()> {
        let mut keys = vec![
            consts::redis_state_key(&self.stream_id, &self.variant_id),
            consts::redis_init_key(&self.stream_id, &self.variant_id),
        ];

        let lower_bound = (self.redis_state.current_segment_idx() as i32 - 4).max(0) as u32;

        keys.extend(
            (lower_bound..self.redis_state.current_segment_idx() + 1).flat_map(|idx| {
                [
                    consts::redis_segment_state_key(&self.stream_id, &self.variant_id, idx),
                    consts::redis_segment_data_key(&self.stream_id, &self.variant_id, idx),
                ]
            }),
        );

        for key in keys.iter() {
            let _: RedisValue = redis
                .expire(key, consts::ACTIVE_EXPIRE_SECONDS)
                .await
                .context("failed to expire redis expire")?;
        }

        for key in self
            .segment_state
            .keys()
            .filter(|idx| **idx < lower_bound)
            .copied()
            .collect::<Vec<_>>()
            .into_iter()
            .flat_map(|idx| {
                self.segment_state.remove(&idx);
                [
                    consts::redis_segment_state_key(&self.stream_id, &self.variant_id, idx),
                    consts::redis_segment_data_key(&self.stream_id, &self.variant_id, idx),
                ]
            })
        {
            let _: RedisValue = redis
                .expire(key, consts::INACTIVE_EXPIRE_SECONDS)
                .await
                .context("failed to expire redis key")?;
        }

        Ok(())
    }

    fn generate_playlist(&mut self) -> Result<()> {
        let mut playlist = String::new();

        let oldest_segment_idx = (self.redis_state.current_segment_idx() as i32
            - consts::ACTIVE_SEGMENT_COUNT as i32)
            .max(0) as u32;
        let oldest_fragment_display_idx = (self.redis_state.current_segment_idx() as i32
            - consts::ACTIVE_FRAGMENT_SEGMENT_COUNT as i32)
            .max(0) as u32;
        let newest_segment_idx = self.redis_state.current_segment_idx()
            + if self.redis_state.current_fragment_idx() == 0 {
                0
            } else {
                1
            };

        let mut discontinuity_sequence = self.redis_state.discontinuity_sequence() as i32;
        let mut segment_data = String::new();

        // According to spec this should never change.
        // But we will just keep it changing to the largest fragment duration in this set of segments.
        // However, this should be a good enough approximation.
        // Baring that the framerate is not less than 8fps. (which is unlikely)
        // If it is less than 8fps, then it will automatically increase to the longest fragment duration.
        let mut longest_fragment_duration: f64 = consts::FRAGMENT_CUT_TARGET_DURATION;

        for idx in oldest_segment_idx..newest_segment_idx {
            let Some((segment, _)) = self.segment_state.get(&idx) else {
                return Err(anyhow::anyhow!("missing segment state: {}", idx));
            };

            if segment.discontinuity() {
                discontinuity_sequence -= 1;
                segment_data.push_str("#EXT-X-DISCONTINUITY\n");
            }

            let track_1_timescale = self.redis_state.track_timescale(0).unwrap_or(1);

            let mut total_duration = 0;
            for (f_idx, fragment) in segment.fragments().iter().enumerate() {
                total_duration += fragment.duration;

                longest_fragment_duration = longest_fragment_duration
                    .max(fragment.duration as f64 / track_1_timescale as f64);

                if idx >= oldest_fragment_display_idx {
                    segment_data.push_str(&format!(
                        "#EXT-X-PART:DURATION={:.5},URI=\"{}.{}.mp4\"{}\n",
                        fragment.duration as f64 / track_1_timescale as f64,
                        idx,
                        f_idx,
                        if fragment.keyframe {
                            ",INDEPENDENT=YES"
                        } else {
                            ""
                        }
                    ));
                }
            }

            let segment_duration = total_duration as f64 / track_1_timescale as f64;
            if segment_duration > self.redis_state.longest_segment() {
                self.redis_state.set_longest_segment(segment_duration);
            }

            if segment.ready() {
                segment_data.push_str(&format!(
                    "#EXT-X-PROGRAM-DATE-TIME:{}\n",
                    segment
                        .timestamp()
                        .to_rfc3339_opts(SecondsFormat::Millis, true)
                ));

                segment_data.push_str(&format!("#EXTINF:{:.5},\n", segment_duration));
                segment_data.push_str(&format!("{}.mp4\n", idx));
            }
        }

        segment_data.push_str(&format!(
            "#EXT-X-PRELOAD-HINT:TYPE=PART,URI=\"{}.{}.mp4\"\n",
            self.redis_state.current_segment_idx(),
            self.redis_state.current_fragment_idx()
        ));

        playlist.push_str("#EXTM3U\n");
        playlist.push_str(&format!(
            "#EXT-X-TARGETDURATION:{}\n",
            self.redis_state.longest_segment().ceil() as u32 * 2,
        ));
        playlist.push_str("#EXT-X-VERSION:9\n");
        playlist.push_str(&format!(
            "#EXT-X-SERVER-CONTROL:CAN-BLOCK-RELOAD=YES,PART-HOLD-BACK={:.5}\n",
            longest_fragment_duration * 2.0
        ));
        playlist.push_str(&format!(
            "#EXT-X-PART-INF:PART-TARGET={:.5}\n",
            longest_fragment_duration
        ));
        playlist.push_str(&format!("#EXT-X-MEDIA-SEQUENCE:{}\n", oldest_segment_idx));
        playlist.push_str(&format!(
            "#EXT-X-DISCONTINUITY-SEQUENCE:{}\n",
            discontinuity_sequence.max(0)
        ));

        playlist.push_str("#EXT-X-MAP:URI=\"init.mp4\"\n");

        playlist.push_str(&segment_data);

        playlist.push('\n');

        for rendition in self
            .renditions
            .renditions()
            .into_iter()
            .filter(|rendition| rendition.id != self.variant_id)
        {
            playlist.push_str(&format!(
                "#EXT-X-RENDITION-REPORT:URI=\"../{}/index.m3u8\",LAST-MSN={},LAST-PART={}\n",
                rendition.id, rendition.last_msn, rendition.last_part
            ));
        }

        self.redis_state.set_playlist(playlist);

        Ok(())
    }
}
