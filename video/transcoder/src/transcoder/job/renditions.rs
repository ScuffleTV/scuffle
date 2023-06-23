use std::{collections::HashMap, sync::atomic::AtomicU32};

#[derive(Debug, Default)]
pub struct RenditionMap {
    map: HashMap<String, AtomicRendition>,
}

impl RenditionMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn renditions(&self) -> Vec<Rendition> {
        self.map
            .iter()
            .map(|(id, r)| Rendition {
                id: id.clone(),
                last_msn: r.last_msn.load(std::sync::atomic::Ordering::Relaxed),
                last_part: r.last_part.load(std::sync::atomic::Ordering::Relaxed),
            })
            .collect()
    }

    pub fn set(&self, id: &str, last_msn: u32, last_part: u32) -> bool {
        if let Some(r) = self.map.get(id) {
            r.last_msn
                .store(last_msn, std::sync::atomic::Ordering::Relaxed);
            r.last_part
                .store(last_part, std::sync::atomic::Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    pub fn insert(&mut self, id: String) {
        self.map.insert(id, AtomicRendition::default());
    }
}

#[derive(Debug, Default)]
struct AtomicRendition {
    last_msn: AtomicU32,
    last_part: AtomicU32,
}

pub struct Rendition {
    pub id: String,
    pub last_msn: u32,
    pub last_part: u32,
}
