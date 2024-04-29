#[cfg(feature = "pprof-heap")]
mod heap;

#[cfg(feature = "pprof-cpu")]
mod cpu;

#[cfg(feature = "pprof-heap")]
pub use heap::Heap;

#[cfg(feature = "pprof-cpu")]
pub use cpu::Cpu;
