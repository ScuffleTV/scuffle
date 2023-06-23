use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    rc::Rc,
};

use wasm_bindgen::JsValue;
use web_sys::HtmlVideoElement;

use super::{
    events::{EventManifestLoaded, EventManager, EventVariantChange, UserEvent, EventAbrChange},
    track::{Track, Variant},
};

pub struct PlayerInner {
    url: String,
    low_latency: bool,
    abr_enabled: bool,
    is_master_playlist: bool,
    video_element: Option<HtmlVideoElement>,
    tracks: Vec<Track>,
    variants: Vec<Variant>,
    em: EventManager,

    active_group_track_ids: Vec<u32>,

    active_variant_id: u32,
    next_variant_id: Option<NextVariant>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NextVariant {
    Switch(u32),
    Force(u32),
}

impl NextVariant {
    pub fn variant_id(&self) -> u32 {
        match self {
            Self::Switch(id) | Self::Force(id) => *id,
        }
    }

    pub fn is_force(&self) -> bool {
        matches!(self, Self::Force(_))
    }
}

impl Default for PlayerInner {
    fn default() -> Self {
        Self {
            url: String::new(),
            low_latency: true,
            abr_enabled: true,
            is_master_playlist: false,
            tracks: Vec::new(),
            video_element: None,
            variants: Vec::new(),
            active_group_track_ids: Vec::new(),
            next_variant_id: None,
            active_variant_id: 0,
            em: EventManager::new(),
        }
    }
}

#[derive(Default, Clone)]
pub struct PlayerInnerHolder(Rc<UnsafeCell<PlayerInner>>);

impl Deref for PlayerInnerHolder {
    type Target = PlayerInner;

    fn deref(&self) -> &Self::Target {
        // Safety: PlayerInner does not return any references to its fields. It also requires that the caller does not create references to PlayerInner.
        // Ie. do not manually dereference the holder, or pass a dereferenced value to another function.
        // Always use the PlayerInnerHolder as the type and pass that around cloning it.
        // Without this unsafe block it becomes very complex to interop with the JS side.
        // TLDR:
        //  1. PlayerInner does not return references to its fields.
        //  2. Requires that the caller does not create references to PlayerInner.
        //  3. Always use the PlayerInnerHolder as the type and pass that around cloning it.
        unsafe { &*self.0.get() }
    }
}

impl DerefMut for PlayerInnerHolder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Safety: PlayerInner does not return any references to its fields. It also requires that the caller does not create references to PlayerInner.
        // Ie. do not manually dereference the holder, or pass a dereferenced value to another function.
        // Always use the PlayerInnerHolder as the type and pass that around cloning it.
        // Without this unsafe block it becomes very complex to interop with the JS side.
        // TLDR:
        //  1. PlayerInner does not return references to its fields.
        //  2. Requires that the caller does not create references to PlayerInner.
        //  3. Always use the PlayerInnerHolder as the type and pass that around cloning it.
        unsafe { &mut *self.0.get() }
    }
}

impl PlayerInner {
    pub fn url(&self) -> String {
        self.url.clone()
    }

    pub fn low_latency(&self) -> bool {
        self.low_latency
    }

    pub fn abr_enabled(&self) -> bool {
        self.abr_enabled
    }

    pub fn set_abr_enabled(&mut self, abr_enabled: bool) {
        self.abr_enabled = abr_enabled;
        self.emit_event(EventAbrChange {
            enabled: self.abr_enabled,
            variant_id: None,
            bandwidth: None
        });
    }

    pub fn video_element(&self) -> Option<HtmlVideoElement> {
        self.video_element.clone()
    }

    pub fn set_video_element(&mut self, element: Option<HtmlVideoElement>) {
        self.video_element = element;
    }

    pub fn tracks(&self) -> Vec<Track> {
        self.tracks.clone()
    }

    pub fn variants(&self) -> Vec<Variant> {
        self.variants.clone()
    }

    pub fn set_url(&mut self, url: impl ToString) {
        self.url = url.to_string();
    }

    pub fn set_low_latency(&mut self, low_latency: bool) {
        self.low_latency = low_latency;
    }

    pub fn set_active_variant_id(&mut self, variant_id: u32) {
        self.em.dispatch_event(EventVariantChange {
            old_variant_id: self.active_variant_id,
            variant_id,
        });

        self.active_variant_id = variant_id;
    }

    pub fn active_variant_id(&self) -> u32 {
        self.active_variant_id
    }

    pub fn set_active_group_track_ids(&mut self, track_ids: Vec<u32>) {
        self.active_group_track_ids = track_ids;
    }

    pub fn set_next_variant_id(&mut self, variant_id: Option<NextVariant>) {
        self.next_variant_id = variant_id;
    }

    pub fn next_variant_id(&self) -> Option<NextVariant> {
        self.next_variant_id
    }

    pub fn set_tracks(
        &mut self,
        tracks: Vec<Track>,
        variants: Vec<Variant>,
        master_playlist: bool,
    ) {
        self.tracks = tracks;
        self.is_master_playlist = master_playlist;
        self.variants = variants;

        self.emit_event(EventManifestLoaded {
            is_master_playlist: self.is_master_playlist,
            variants: self.variants.clone(),
            tracks: self.tracks.clone(),
        })
    }

    pub fn emit_event(&mut self, evt: impl Into<UserEvent>) {
        self.em.dispatch_event(evt)
    }

    pub fn add_event_listener(&mut self, event: &str, f: JsValue, once: bool) {
        self.em.add_event_listener(event, f, once);
    }

    pub fn remove_event_listener(&mut self, event: &str, f: JsValue) {
        self.em.remove_event_listener(event, f);
    }
}
