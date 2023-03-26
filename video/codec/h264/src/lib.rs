mod config;
mod sps;

pub use self::{
    config::{AVCDecoderConfigurationRecord, AvccExtendedConfig},
    sps::{ColorConfig, Sps, SpsExtended},
};

#[cfg(test)]
mod tests;
