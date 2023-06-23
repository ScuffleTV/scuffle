use std::{cell::RefCell, rc::Rc, sync::atomic::AtomicUsize};

use super::fetch::Metrics;

#[derive(Debug)]
struct Report {
    total_bytes: u32,
    bandwidth: u32,
}

#[derive(Clone)]
pub struct Bandwidth {
    bandwidth: Rc<RefCell<Vec<Report>>>,
    max_count: Rc<AtomicUsize>,
}

impl Default for Bandwidth {
    fn default() -> Self {
        Self {
            bandwidth: Rc::new(RefCell::new(Vec::new())),
            max_count: Rc::new(AtomicUsize::new(10)),
        }
    }
}

impl std::fmt::Debug for Bandwidth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bandwidth")
            .field("bandwidth", &self.get())
            .field("reports", &self.bandwidth.borrow().as_slice())
            .finish()
    }
}

impl Bandwidth {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self) -> Option<u32> {
        let inner = self.bandwidth.borrow();
        if inner.is_empty() {
            return None;
        }

        let total = inner.iter().map(|r| r.total_bytes).sum::<u32>() as f64;
        Some(
            inner
                .iter()
                .map(|r| r.bandwidth as f64 * r.total_bytes as f64 / total)
                .sum::<f64>() as u32,
        )
    }

    pub fn set_max_count(&self, max_count: usize) {
        self.max_count
            .store(max_count, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn report_download(&self, metrics: &Metrics) {
        if metrics.download_size == 0 {
            return;
        }

        let real_download_time = metrics.download_time / 1000.0;
        let real_bandwidth = metrics.download_size as f64 / real_download_time;

        {
            let mut inner = self.bandwidth.borrow_mut();
            let size = inner.len();
            let max_count = self.max_count.load(std::sync::atomic::Ordering::Relaxed);
            if size > max_count {
                inner.drain(0..(size - max_count));
            }

            inner.push(Report {
                total_bytes: metrics.download_size,
                bandwidth: real_bandwidth as u32,
            });
        }
    }
}
