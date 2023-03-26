use bytes::Bytes;
use h264::{AVCDecoderConfigurationRecord, AvccExtendedConfig};
use mp4::{
    types::{
        avc1::Avc1,
        avcc::AvcC,
        hdlr::{HandlerType, Hdlr},
        mdat::Mdat,
        mdhd::Mdhd,
        mdia::Mdia,
        mfhd::Mfhd,
        minf::Minf,
        moof::Moof,
        moov::Moov,
        mvex::Mvex,
        mvhd::Mvhd,
        stbl::Stbl,
        stco::Stco,
        stsc::Stsc,
        stsd::{SampleEntry, Stsd, VisualSampleEntry},
        stts::Stts,
        tfdt::Tfdt,
        tfhd::Tfhd,
        tkhd::Tkhd,
        traf::Traf,
        trak::Trak,
        trex::Trex,
        trun::{Trun, TrunSample, TrunSampleFlag},
        vmhd::Vmhd,
    },
    BoxType, DynBox,
};

pub struct VideoFactory {
    timescale: u32,
    sequence_number: u32,
}

impl VideoFactory {
    pub fn new(timescale: u32) -> Self {
        Self {
            sequence_number: 0,
            timescale,
        }
    }

    pub fn moov(&self) -> Moov {
        Moov::new(
            Mvhd::new(0, 0, 1000, 0, 2),
            vec![Trak::new(
                Tkhd::new(
                    0,
                    0,
                    1,
                    0,
                    Some((2, 2)),
                ),
                None,
                Mdia::new(
                    Mdhd::new(0, 0, self.timescale, 0),
                    Hdlr::new(HandlerType::Vide, "".to_string()),
                    Minf::new(
                        Stbl::new(
                            Stsd::new(vec![DynBox::Avc1(Avc1::new(
                                SampleEntry::new(VisualSampleEntry::new(2, 2, None)),
                                AvcC::new(AVCDecoderConfigurationRecord {
                                    configuration_version: 1,
                                    profile_indication: 100,
                                    profile_compatibility: 0,
                                    level_indication: 10,
                                    length_size_minus_one: 3,
                                    sps: vec![Bytes::from_static(b"gd\0\n\xac\xd9_\x88\x88\xc0D\0\0\x03\0\x04\0\0\x03\0\x08<H\x96X")],
                                    pps: vec![Bytes::from_static(b"h\xeb\xe3\xcb\"\xc0")],
                                    extended_config: Some(AvccExtendedConfig {
                                        chroma_format: 1,
                                        bit_depth_luma_minus8: 0,
                                        bit_depth_chroma_minus8: 0,
                                        sequence_parameter_set_ext: vec![],
                                    }),
                                }),
                                None,
                            ))]),
                            Stts::new(vec![]),
                            Stsc::new(vec![]),
                            Stco::new(vec![]),
                            None,
                        ),
                        Some(Vmhd::new()),
                        None,
                    ),
                )
            )],
            Some(Mvex::new(vec![Trex::new(1)], None)),
        )
    }

    pub fn moof_mdat(&mut self, decode_time: u64, duration: u32) -> (Moof, Mdat) {
        self.sequence_number += 1;

        let mdat = Mdat::new(vec![Bytes::from_static(b"\0\0\x02\xad\x06\x05\xff\xff\xa9\xdcE\xe9\xbd\xe6\xd9H\xb7\x96,\xd8 \xd9#\xee\xefx264 - core 163 r3060 5db6aa6 - H.264/MPEG-4 AVC codec - Copyleft 2003-2021 - http://www.videolan.org/x264.html - options: cabac=1 ref=3 deblock=1:0:0 analyse=0x3:0x113 me=hex subme=7 psy=1 psy_rd=1.00:0.00 mixed_ref=1 me_range=16 chroma_me=1 trellis=1 8x8dct=1 cqm=0 deadzone=21,11 fast_pskip=1 chroma_qp_offset=-2 threads=1 lookahead_threads=1 sliced_threads=0 nr=0 decimate=1 interlaced=0 bluray_compat=0 constrained_intra=0 bframes=3 b_pyramid=2 b_adapt=1 b_bias=0 direct=1 weightb=1 open_gop=0 weightp=2 keyint=250 keyint_min=1 scenecut=40 intra_refresh=0 rc_lookahead=40 rc=crf mbtree=1 crf=23.0 qcomp=0.60 qpmin=0 qpmax=69 qpstep=4 ip_ratio=1.40 aq=1:1.00\0\x80\0\0\0\x10e\x88\x84\0\x15\xff\xfe\xf7\xc9\xef\xc0\xa6\xeb\xdb\xdf\x81")]);

        let mut moof = Moof::new(
            Mfhd::new(self.sequence_number),
            vec![Traf::new(
                Tfhd::new(
                    1,
                    None,
                    None,
                    Some(duration),
                    Some(mdat.primitive_size() as u32),
                    Some(TrunSampleFlag {
                        reserved: 0,
                        is_leading: 0,
                        sample_depends_on: 2,
                        sample_is_depended_on: 0,
                        sample_has_redundancy: 0,
                        sample_padding_value: 0,
                        sample_is_non_sync_sample: false,
                        sample_degradation_priority: 0,
                    }),
                ),
                Some(Trun::new(
                    vec![TrunSample {
                        composition_time_offset: None,
                        duration: None,
                        size: None,
                        flags: None,
                    }],
                    None,
                )),
                Some(Tfdt::new(decode_time)),
            )],
        );

        let offset = moof.size() + 8;

        moof.traf[0].trun.as_mut().unwrap().data_offset = Some(offset as i32);

        (moof, mdat)
    }
}
