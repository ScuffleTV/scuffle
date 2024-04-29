use std::io::Write;

use anyhow::Context;
use flate2::{write::GzEncoder, Compression};
use pprof::protos::Message;

pub struct Cpu(pprof::ProfilerGuardBuilder);

impl Cpu {
    pub fn new<S: AsRef<str>>(frequency: i32, blocklist: &[S]) -> Self {
        Self(
            pprof::ProfilerGuardBuilder::default()
                .frequency(frequency)
                .blocklist(blocklist),
        )
    }

    /// Capture a pprof profile for the given duration.
    /// The profile is compressed using gzip.
    /// The profile can be analyzed using the `pprof` tool.
    /// Warning: This method is blocking and may take a long time to complete. It is recommended to run it in a separate thread.
    pub fn capture(&self, duration: std::time::Duration) -> anyhow::Result<Vec<u8>> {
        let profiler = self
            .0
            .clone()
            .build()
            .context("failed to build pprof profiler")?;

        std::thread::sleep(duration);

        let report = profiler
            .report()
            .build()
            .context("failed to build pprof report")?;

        let pprof = report.pprof().context("failed to build pprof profile")?;

        let mut gz = GzEncoder::new(Vec::new(), Compression::default());
        gz.write_all(&pprof.encode_to_vec())
            .context("failed to compress pprof profile")?;
        gz.finish()
            .context("failed to finish compressing pprof profile")
    }
}
