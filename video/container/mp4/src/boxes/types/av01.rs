use std::io;

use bytes::{Buf, Bytes};

use crate::boxes::{header::BoxHeader, traits::BoxType, DynBox};

use super::{
    av1c::Av1C,
    btrt::Btrt,
    stsd::{SampleEntry, VisualSampleEntry},
};

#[derive(Debug, Clone, PartialEq)]
/// AV1 Codec Box
/// https://aomediacodec.github.io/av1-isobmff/#av1sampleentry-section
pub struct Av01 {
    pub header: BoxHeader,
    pub visual_sample_entry: SampleEntry<VisualSampleEntry>,
    pub av1c: Av1C,
    pub btrt: Option<Btrt>,
    pub unknown: Vec<DynBox>,
}

impl Av01 {
    pub fn new(
        visual_sample_entry: SampleEntry<VisualSampleEntry>,
        av1c: Av1C,
        btrt: Option<Btrt>,
    ) -> Self {
        Self {
            header: BoxHeader::new(Self::NAME),
            visual_sample_entry,
            av1c,
            btrt,
            unknown: Vec::new(),
        }
    }
}

impl BoxType for Av01 {
    const NAME: [u8; 4] = *b"av01";

    fn demux(header: BoxHeader, data: Bytes) -> io::Result<Self> {
        let mut reader = io::Cursor::new(data);

        let mut visual_sample_entry = SampleEntry::<VisualSampleEntry>::demux(&mut reader)?;

        let mut av1c = None;
        let mut btrt = None;
        let mut unknown = Vec::new();

        while reader.has_remaining() {
            let dyn_box = DynBox::demux(&mut reader)?;
            match dyn_box {
                DynBox::Av1C(b) => {
                    av1c = Some(b);
                }
                DynBox::Btrt(b) => {
                    btrt = Some(b);
                }
                DynBox::Clap(b) => {
                    visual_sample_entry.extension.clap = Some(b);
                }
                DynBox::Pasp(b) => {
                    visual_sample_entry.extension.pasp = Some(b);
                }
                DynBox::Colr(b) => {
                    visual_sample_entry.extension.colr = Some(b);
                }
                _ => {
                    unknown.push(dyn_box);
                }
            }
        }

        let av1c = av1c.ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "trak box is missing tkhd box")
        })?;

        Ok(Self {
            header,
            visual_sample_entry,
            av1c,
            btrt,
            unknown,
        })
    }

    fn primitive_size(&self) -> u64 {
        self.visual_sample_entry.size()
            + self.av1c.size()
            + self.btrt.as_ref().map(|b| b.size()).unwrap_or(0)
            + self.unknown.iter().map(|b| b.size()).sum::<u64>()
    }

    fn primitive_mux<T: io::Write>(&self, writer: &mut T) -> io::Result<()> {
        self.visual_sample_entry.mux(writer)?;
        self.av1c.mux(writer)?;
        if let Some(btrt) = &self.btrt {
            btrt.mux(writer)?;
        }
        for unknown in &self.unknown {
            unknown.mux(writer)?;
        }
        Ok(())
    }
}
