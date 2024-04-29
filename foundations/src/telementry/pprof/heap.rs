use anyhow::Context;

#[allow(non_upper_case_globals)]
#[export_name = "malloc_conf"]
#[cfg(not(feature = "disable-jemalloc-config"))]
pub static malloc_conf: &[u8] = b"prof:true,prof_active:true,lg_prof_sample:19,abort_conf:true\0";

pub struct Heap;

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}

impl Heap {
    pub fn new() -> Self {
        Self
    }

    /// Capture a heap profile for the given duration.
    /// The profile can be analyzed using the `pprof` tool.
    /// Warning: This method is blocking and may take a long time to complete. It is recommended to run it in a separate thread.
    pub fn capture(&mut self) -> anyhow::Result<Vec<u8>> {
        let mut profiler = jemalloc_pprof::PROF_CTL
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("jemalloc profiling is not available"))?
            .blocking_lock();

        if !profiler.activated() {
            // profiler.deactivate().context("failed to deactivate jemalloc profiling")?;
            profiler
                .activate()
                .context("failed to activate jemalloc profiling")?;
        }

        profiler
            .dump_pprof()
            .context("failed to dump jemalloc pprof profile")
    }
}
