#[cfg(feature = "pprof-cpu")]
mod cpu;

#[cfg(feature = "pprof-cpu")]
pub use cpu::Cpu;
