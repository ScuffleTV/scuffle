use std::{fmt::Debug, io};

use byteorder::WriteBytesExt;
use bytes::Bytes;
use paste::paste;

pub mod header;
mod traits;
pub mod types;

#[macro_use]
mod macros;

use header::BoxHeader;
pub use traits::BoxType;

use crate::boxes::types::{
    av01::Av01, av1c::Av1C, avc1::Avc1, avcc::AvcC, btrt::Btrt, clap::Clap, co64::Co64, colr::Colr,
    ctts::Ctts, dinf::Dinf, dref::Dref, edts::Edts, elst::Elst, esds::Esds, ftyp::Ftyp, hdlr::Hdlr,
    hev1::Hev1, hmhd::Hmhd, hvcc::HvcC, mdat::Mdat, mdhd::Mdhd, mdia::Mdia, mehd::Mehd, mfhd::Mfhd,
    minf::Minf, moof::Moof, moov::Moov, mp4a::Mp4a, mvex::Mvex, mvhd::Mvhd, nmhd::Nmhd, opus::Opus,
    padb::Padb, pasp::Pasp, sbgp::Sbgp, sdtp::Sdtp, smhd::Smhd, stbl::Stbl, stco::Stco, stdp::Stdp,
    stsc::Stsc, stsd::Stsd, stsh::Stsh, stss::Stss, stsz::Stsz, stts::Stts, stz2::Stz2, subs::Subs,
    tfdt::Tfdt, tfhd::Tfhd, tkhd::Tkhd, traf::Traf, trak::Trak, trex::Trex, trun::Trun, url::Url,
    vmhd::Vmhd,
};

#[rustfmt::skip]
impl_box!(
    Ftyp, Moov, Mvhd, Mvex, Trak, Trex,
    Mehd, Mdia, Tkhd, Edts, Elst, Mdhd,
    Minf, Hdlr, Dinf, Stbl, Hmhd, Nmhd,
    Smhd, Vmhd, Dref, Stsd, Stsz, Stsc,
    Stco, Co64, Stts, Stss, Stz2, Stsh,
    Ctts, Stdp, Sbgp, Subs, Padb, Sdtp,
    Url, Avc1, Clap, Pasp, AvcC, Btrt,
    Mp4a, Esds, Moof, Mfhd, Traf, Tfhd,
    Tfdt, Trun, Mdat, Av01, Av1C, Colr,
    Hev1, HvcC, Opus,
);
