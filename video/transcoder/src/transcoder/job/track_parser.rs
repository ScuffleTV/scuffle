use std::io;

use anyhow::anyhow;
use async_stream::stream;
use bytes::{Buf, Bytes, BytesMut};
use bytesio::bytes_reader::BytesCursor;
use futures_util::{Stream, StreamExt};
use mp4::{
    types::{moov::Moov, trun::TrunSample},
    DynBox,
};

#[derive(Debug, Clone)]
pub enum TrackOut {
    // a Ftyp and Moov box are always sent at the start of a stream
    Moov(Moov),
    // A moof and mdat box are sent for each segment
    Sample(TrackSample),
}

#[derive(Debug, Clone)]
pub struct TrackSample {
    pub duration: u32,
    pub keyframe: bool,
    pub sample: TrunSample,
    pub data: Bytes,
}

pub fn track_parser(
    mut input: impl Stream<Item = io::Result<Bytes>> + Unpin,
) -> impl Stream<Item = io::Result<TrackOut>> {
    stream! {
        let mut buffer = BytesMut::new();

        // Main loop for parsing the stream
        while let Some(data) = input.next().await {
            buffer.extend_from_slice(&data?);
            let mut cursor = io::Cursor::new(buffer.split().freeze());

            while cursor.has_remaining() {
                let position = cursor.position() as usize;
                let b = match mp4::DynBox::demux(&mut cursor) {
                    Ok(b) => b,
                    Err(e) => {
                        if e.kind() == io::ErrorKind::UnexpectedEof {
                            // We need more data to parse this box
                            cursor.set_position(position as u64);
                            break;
                        } else {
                            yield Err(e);
                            return;
                        }
                    }
                };

                match b {
                    mp4::DynBox::Moov(moov) => {
                        if moov.traks.len() != 1 {
                            yield Err(io::Error::new(io::ErrorKind::InvalidData, anyhow!("moov box must have exactly one trak box")));
                            return;
                        }

                        yield Ok(TrackOut::Moov(moov));
                    },
                    mp4::DynBox::Moof(moof) => {
                        if moof.traf.len() != 1 {
                            yield Err(io::Error::new(io::ErrorKind::InvalidData, anyhow!("moof box must have exactly one traf box")));
                            return;
                        }

                        let traf = &moof.traf[0];
                        let trun = traf.trun.as_ref().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, anyhow!("traf box must have a trun box")))?;
                        let tfhd = &traf.tfhd;

                        let samples = trun.samples.iter().enumerate().map(|(idx, sample)| {
                            let mut sample = sample.clone();
                            sample.duration = sample.duration.or(tfhd.default_sample_duration);
                            sample.size = sample.size.or(tfhd.default_sample_size);
                            sample.flags = Some(sample.flags.or(if idx == 0 { trun.first_sample_flags } else { None }).or(tfhd.default_sample_flags).unwrap_or_default());
                            sample.composition_time_offset = Some(sample.composition_time_offset.unwrap_or_default());
                            sample
                        });

                        // Get the mdat box
                        let mdat = match mp4::DynBox::demux(&mut cursor) {
                            Ok(DynBox::Mdat(mdat)) => mdat,
                            Ok(_) => {
                                yield Err(io::Error::new(io::ErrorKind::InvalidData, anyhow!("moof box must be followed by an mdat box")));
                                return;
                            },
                            Err(e) => {
                                if e.kind() == io::ErrorKind::UnexpectedEof {
                                    // We need more data to parse this box
                                    cursor.set_position(position as u64);
                                    break;
                                } else {
                                    yield Err(e);
                                    return;
                                }
                            }
                        };

                        if mdat.data.len() != 1 {
                            yield Err(io::Error::new(io::ErrorKind::InvalidData, anyhow!("mdat box must have exactly one data box")));
                            return;
                        }

                        let mut mdat_cursor = io::Cursor::new(mdat.data[0].clone());
                        for sample in samples {
                            let data = if let Some(size) = sample.size {
                                mdat_cursor.read_slice(size as usize).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, anyhow!("mdat data size not big enough for sample: {}", e)))?
                            } else {
                                mdat_cursor.get_remaining()
                            };

                            yield Ok(TrackOut::Sample(TrackSample {
                                duration: sample.duration.unwrap_or_default(),
                                keyframe: sample.flags.map(|f| f.sample_depends_on == 2).unwrap_or_default(),
                                sample,
                                data,
                            }));
                        }
                    },
                     _ => {},
                }
            }

            buffer.extend_from_slice(&cursor.get_remaining());
        }
    }
}
