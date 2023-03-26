use std::{
    cell::{Cell, Ref, RefCell, RefMut},
    panic::Location,
    rc::Rc,
};

use wasm_bindgen::{JsCast, JsValue};
use web_sys::HtmlVideoElement;

use super::{
    events::{EventError, EventManifestLoaded, OnErrorFunction, OnManifestLoadedFunction},
    track::Track,
};

pub struct PlayerInner {
    url: String,
    low_latency: bool,
    is_master_playlist: bool,
    abr_estimate: Option<u32>,
    video_element: Option<HtmlVideoElement>,
    on_error: Option<OnErrorFunction>,
    on_manifest_loaded: Option<OnManifestLoadedFunction>,
    tracks: Vec<Track>,

    active_track_id: u32,
    active_reference_track_ids: Vec<u32>,

    next_track_id: Option<NextTrack>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NextTrack {
    Switch(u32),
    Force(u32),
}

impl NextTrack {
    pub fn track_id(&self) -> u32 {
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
            is_master_playlist: false,
            abr_estimate: None,
            on_error: None,
            on_manifest_loaded: None,
            tracks: Vec::new(),
            video_element: None,
            active_reference_track_ids: Vec::new(),
            active_track_id: 0,
            next_track_id: None,
        }
    }
}

#[derive(Default, Clone)]
pub struct PlayerInnerHolder {
    inner: Rc<RefCell<PlayerInner>>,
    previous_holder: Cell<Option<&'static Location<'static>>>,
}

impl PlayerInnerHolder {
    const AQUIRE_ERROR: &'static str = r#"We failed to borrow the inner state, this is a bug!
Likely caused by holidng a reference to the inner state across an await point.
If you see this error, please file a bug report at https://github.com/scuffletv/scuffle"#;

    #[track_caller]
    pub fn aquire(&self) -> Ref<'_, PlayerInner> {
        let Ok(inner) = self.inner.try_borrow() else {
            tracing::error!("{}\nPrevious hold at: {}\nNew hold at: {}", Self::AQUIRE_ERROR, self.previous_holder.get().unwrap(), Location::caller());
            unreachable!("{}", Self::AQUIRE_ERROR)
        };

        self.previous_holder.set(Some(Location::caller()));

        inner
    }

    #[track_caller]
    pub fn aquire_mut(&self) -> RefMut<'_, PlayerInner> {
        let Ok(inner) = self.inner.try_borrow_mut() else {
            tracing::error!("{}\nPrevious hold at: {}\nNew hold at: {}", Self::AQUIRE_ERROR, self.previous_holder.get().unwrap(), Location::caller());
            unreachable!("{}", Self::AQUIRE_ERROR)
        };

        self.previous_holder.set(Some(Location::caller()));

        inner
    }
}

impl PlayerInner {
    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn low_latency(&self) -> bool {
        self.low_latency
    }

    pub fn video_element(&self) -> Option<HtmlVideoElement> {
        self.video_element.clone()
    }

    pub fn set_video_element(&mut self, element: Option<HtmlVideoElement>) {
        self.video_element = element;
    }

    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    pub fn set_url(&mut self, url: impl ToString) {
        self.url = url.to_string();
    }

    pub fn set_on_error(&mut self, f: Option<OnErrorFunction>) {
        self.on_error = f;
    }

    pub fn set_on_manifest_loaded(&mut self, f: Option<OnManifestLoadedFunction>) {
        self.on_manifest_loaded = f;
    }

    pub fn on_error(&self) -> Option<OnErrorFunction> {
        self.on_error
            .as_ref()
            .map(|s| s.dyn_ref::<JsValue>().unwrap().clone().unchecked_into())
    }

    pub fn on_manifest_loaded(&self) -> Option<OnManifestLoadedFunction> {
        self.on_manifest_loaded
            .as_ref()
            .map(|s| s.dyn_ref::<JsValue>().unwrap().clone().unchecked_into())
    }

    pub fn set_low_latency(&mut self, low_latency: bool) {
        self.low_latency = low_latency;
    }

    pub fn set_abr_estimate(&mut self, abr_estimate: Option<u32>) {
        self.abr_estimate = abr_estimate;
    }

    pub fn set_active_track_id(&mut self, track_id: u32) {
        self.active_track_id = track_id;
    }

    pub fn active_track_id(&self) -> u32 {
        self.active_track_id
    }

    pub fn set_active_reference_track_ids(&mut self, groups: Vec<u32>) {
        self.active_reference_track_ids = groups;
    }

    pub fn set_next_track_id(&mut self, track_id: Option<NextTrack>) {
        self.next_track_id = track_id;
    }

    pub fn next_track_id(&self) -> Option<NextTrack> {
        self.next_track_id
    }

    pub fn set_tracks(&mut self, tracks: Vec<Track>, master_playlist: bool) -> impl FnOnce() {
        self.tracks = tracks;
        self.is_master_playlist = master_playlist;

        self.send_manifest_loaded(EventManifestLoaded {
            is_master_playlist: self.is_master_playlist,
            tracks: self.tracks.clone(),
        })
    }

    pub fn send_error(&self, error: EventError) -> impl FnOnce() {
        let js_fn = self.on_error();
        move || {
            if let Some(f) = js_fn {
                if let Err(err) = f.call(JsValue::null(), error) {
                    tracing::error!("Error in on_error callback: {:?}", err);
                }
            }
        }
    }

    pub fn send_manifest_loaded(&self, evt: EventManifestLoaded) -> impl FnOnce() {
        let js_fn = self.on_manifest_loaded();
        move || {
            if let Some(f) = js_fn {
                if let Err(err) = f
                    .dyn_ref::<JsValue>()
                    .unwrap()
                    .clone()
                    .unchecked_into::<OnManifestLoadedFunction>()
                    .call(JsValue::null(), evt)
                {
                    tracing::error!("Error in on_manifest_loaded callback: {:?}", err);
                }
            }
        }
    }
}
