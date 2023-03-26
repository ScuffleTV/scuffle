use tokio::sync::broadcast;
use tsify::JsValueSerdeExt;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlVideoElement;

use self::{
    events::{OnErrorFunction, OnManifestLoadedFunction},
    inner::{NextTrack, PlayerInnerHolder},
    runner::PlayerRunner,
};

mod blank;
mod events;
mod fetch;
mod inner;
mod runner;
mod track;
mod util;

#[wasm_bindgen]
pub struct Player {
    shutdown_sender: broadcast::Sender<()>,
    inner: PlayerInnerHolder,
}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "Track[]")]
    pub type VectorTracks;
}

#[wasm_bindgen]
impl Player {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let inner = PlayerInnerHolder::default();
        let (shutdown_sender, _) = broadcast::channel(128);

        Self {
            shutdown_sender,
            inner,
        }
    }

    #[wasm_bindgen(setter = lowLatency)]
    pub fn set_low_latency(&self, low_latency: bool) {
        self.inner.aquire_mut().set_low_latency(low_latency);
    }

    #[wasm_bindgen(getter = lowLatency)]
    pub fn low_latency(&self) -> bool {
        self.inner.aquire().low_latency()
    }

    pub fn set_abr_estimate(&self, abr_estimate: Option<u32>) {
        self.inner.aquire_mut().set_abr_estimate(abr_estimate);
    }

    pub fn load(&self, url: &str) -> Result<(), JsValue> {
        let mut inner = self.inner.aquire_mut();

        inner.set_url(url);

        if inner.video_element().is_none() {
            return Ok(());
        }

        self.shutdown();
        self.spawn_runner();

        Ok(())
    }

    #[wasm_bindgen(setter = onerror)]
    pub fn set_on_error(&self, f: Option<OnErrorFunction>) {
        self.inner.aquire_mut().set_on_error(f);
    }

    #[wasm_bindgen(getter = onerror)]
    pub fn on_error(&self) -> Option<OnErrorFunction> {
        self.inner.aquire().on_error()
    }

    #[wasm_bindgen(getter = tracks)]
    pub fn tracks(&self) -> VectorTracks {
        self.inner
            .aquire()
            .tracks()
            .iter()
            .map(JsValue::from_serde)
            .collect::<serde_json::Result<js_sys::Array>>()
            .unwrap()
            .unchecked_into()
    }

    #[wasm_bindgen(setter = onmanifestloaded)]
    pub fn set_on_manifest_loaded(&self, f: Option<OnManifestLoadedFunction>) {
        self.inner.aquire_mut().set_on_manifest_loaded(f);
    }

    #[wasm_bindgen(getter = onmanifestloaded)]
    pub fn on_manifest_loaded(&self) -> Option<OnManifestLoadedFunction> {
        self.inner.aquire().on_manifest_loaded()
    }

    pub fn attach(&self, el: HtmlVideoElement) -> Result<(), JsValue> {
        let Ok(element) = el.dyn_into::<web_sys::HtmlVideoElement>() else {
            return Err(JsValue::from_str("element is not a video element"));
        };

        if let Some(el) = self.inner.aquire().video_element() {
            if el.is_same_node(Some(&element)) {
                return Err(JsValue::from_str("element is already attached"));
            }
        }

        self.inner.aquire_mut().set_video_element(Some(element));

        if self.inner.aquire().url().is_empty() {
            return Ok(());
        }

        self.shutdown();
        self.spawn_runner();

        Ok(())
    }

    pub fn shutdown(&self) {
        self.shutdown_sender.send(()).ok();
    }

    /// Gracefully switch to this track id when the current segment is finished.
    #[wasm_bindgen(setter = nextTrackId)]
    pub fn set_next_track_id(&self, track_id: Option<u32>) {
        self.inner
            .aquire_mut()
            .set_next_track_id(track_id.map(NextTrack::Switch))
    }

    /// Get the next track id that will be switched to.
    #[wasm_bindgen(getter = nextTrackId)]
    pub fn next_track_id(&self) -> Option<u32> {
        match self.inner.aquire().next_track_id() {
            Some(NextTrack::Switch(track_id)) | Some(NextTrack::Force(track_id)) => Some(track_id),
            None => None,
        }
    }

    /// Force switch to this track id immediately.
    #[wasm_bindgen(setter = forceTrackId)]
    pub fn set_force_track_id(&self, track_id: Option<u32>) {
        self.inner
            .aquire_mut()
            .set_next_track_id(track_id.map(NextTrack::Force))
    }

    /// Get the track id that will be forced to switch to.
    #[wasm_bindgen(getter = forceTrackId)]
    pub fn force_track_id(&self) -> Option<u32> {
        match self.inner.aquire().next_track_id() {
            Some(NextTrack::Force(track_id)) => Some(track_id),
            _ => None,
        }
    }

    /// Get the current track id.
    #[wasm_bindgen(getter = trackId)]
    pub fn track_id(&self) -> u32 {
        self.inner.aquire().active_track_id()
    }
}

impl Player {
    fn spawn_runner(&self) {
        spawn_local(
            PlayerRunner::new(self.inner.clone(), self.shutdown_sender.subscribe()).start(),
        );
    }
}
