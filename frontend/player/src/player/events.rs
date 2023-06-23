use std::collections::HashMap;

use serde::Serialize;
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use super::track::{Track, Variant};

#[wasm_bindgen(getter_with_clone, inspectable)]
pub struct EventError {
    #[wasm_bindgen(readonly)]
    pub error: JsValue,
}

impl From<JsValue> for EventError {
    fn from(error: JsValue) -> Self {
        Self { error }
    }
}

#[derive(Tsify, Serialize)]
#[tsify(into_wasm_abi)]
pub struct EventManifestLoaded {
    pub is_master_playlist: bool,
    pub tracks: Vec<Track>,
    pub variants: Vec<Variant>,
}

#[derive(Tsify, Serialize)]
#[tsify(into_wasm_abi)]
pub struct EventVariantChange {
    pub variant_id: u32,
    pub old_variant_id: u32,
}

#[derive(Tsify, Serialize)]
#[tsify(into_wasm_abi)]
pub struct EventLowLatency {
    pub low_latency: bool,
}

#[derive(Tsify, Serialize)]
#[tsify(into_wasm_abi)]
pub struct EventAbrChange {
    pub enabled: bool,
    pub variant_id: Option<u32>,
    pub bandwidth: Option<u32>,
}

#[derive(Tsify, Serialize)]
#[tsify(into_wasm_abi)]
pub struct EventLoad {
    pub url: String,
}

pub enum UserEvent {
    Error(EventError),
    Load(EventLoad),
    ManifestLoaded(EventManifestLoaded),
    VariantChange(EventVariantChange),
    LowLatency(EventLowLatency),
    AbrChange(EventAbrChange),
    Shutdown,
}

impl UserEvent {
    fn name(&self) -> &'static str {
        match self {
            Self::Error(_) => "error",
            Self::Load(_) => "load",
            Self::ManifestLoaded(_) => "manifestloaded",
            Self::VariantChange(_) => "variantchange",
            Self::LowLatency(_) => "lowlatency",
            Self::AbrChange(_) => "abrchange",
            Self::Shutdown => "shutdown",
        }
    }

    fn value(self) -> JsValue {
        match self {
            Self::Error(error) => error.into(),
            Self::Load(load) => load.into_js().unwrap().into(),
            Self::ManifestLoaded(manifest) => manifest.into_js().unwrap().into(),
            Self::LowLatency(low_latency) => low_latency.into_js().unwrap().into(),
            Self::AbrChange(abr_change) => abr_change.into_js().unwrap().into(),
            Self::VariantChange(variant_change) => variant_change.into_js().unwrap().into(),
            Self::Shutdown => JsValue::null(),
        }
    }
}

impl From<EventError> for UserEvent {
    fn from(error: EventError) -> Self {
        Self::Error(error)
    }
}

impl From<EventLoad> for UserEvent {
    fn from(load: EventLoad) -> Self {
        Self::Load(load)
    }
}

impl From<EventManifestLoaded> for UserEvent {
    fn from(manifest: EventManifestLoaded) -> Self {
        Self::ManifestLoaded(manifest)
    }
}

impl From<EventLowLatency> for UserEvent {
    fn from(low_latency: EventLowLatency) -> Self {
        Self::LowLatency(low_latency)
    }
}

impl From<EventAbrChange> for UserEvent {
    fn from(abr_change: EventAbrChange) -> Self {
        Self::AbrChange(abr_change)
    }
}

impl From<EventVariantChange> for UserEvent {
    fn from(variant_change: EventVariantChange) -> Self {
        Self::VariantChange(variant_change)
    }
}

struct EventListener {
    f: js_sys::Function,
    once: bool,
}

pub struct EventManager {
    events: HashMap<String, Vec<EventListener>>,
}

impl EventManager {
    pub fn new() -> Self {
        Self {
            events: HashMap::new(),
        }
    }

    pub fn add_event_listener(&mut self, event: &str, f: JsValue, once: bool) {
        let listeners = self.events.entry(event.to_string()).or_insert_with(Vec::new);
        listeners.push(EventListener { f: f.unchecked_into(), once });
    }

    pub fn remove_event_listener(&mut self, event: &str, f: JsValue) {
        if let Some(listeners) = self.events.get_mut(event) {
            listeners.retain(|x| !JsValue::eq(&x.f, &f));
        }

        if let Some(listeners) = self.events.get(event) {
            if listeners.is_empty() {
                self.events.remove(event);
            }
        }
    }

    pub fn dispatch_event(&mut self, event: impl Into<UserEvent>) {
        let event = event.into();
        let name = event.name();
        let evt = event.value();

        if let Some(listeners) = self.events.get_mut(name) {
            let mut remove_listeners = Vec::new();
            for (idx, listener) in listeners.iter().enumerate() {
                let func: &js_sys::Function = listener.f.unchecked_ref();
                if let Err(err) = func.call1(&JsValue::undefined(), &evt) {
                    tracing::error!("event target raised exception: {:?}", err);
                }

                if listener.once {
                    remove_listeners.push(idx);
                }
            }

            for idx in remove_listeners.into_iter().rev() {
                listeners.remove(idx);
            }
        }
    }
}
