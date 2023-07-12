use tokio::sync::broadcast;
use tsify::JsValueSerdeExt;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlVideoElement;

use self::{
    inner::{NextVariant, PlayerInnerHolder},
    runner::PlayerRunner,
};

mod bandwidth;
mod blank;
mod events;
mod fetch;
mod inner;
mod runner;
mod track;
mod util;

#[wasm_bindgen(typescript_custom_section)]
const _: &'static str = r#"
type PlayerEvents = {
    error: (evt: EventError) => void;
    load: (evt: EventLoad) => void;
    manifestloaded: (evt: EventManifestLoaded) => void;
    variantchange: (evt: EventVariantChange) => void;
    lowlatency: (evt: EventLowLatency) => void;
    abrchange: (evt: EventAbrChange) => void;
    shutdown: () => void;
};

class Player {
    toJSON(): Object;
    toString(): string;

    constructor();

    load(url: string): void;
    attach(el: HTMLVideoElement): void;
    detach(): void;
    shutdown(): void;

    on<K extends keyof PlayerEvents>(event: K, f: PlayerEvents[K]): void;
    removeListener<K extends keyof PlayerEvents>(event: K, f: PlayerEvents[K]): void;
    once<K extends keyof PlayerEvents>(event: K, f: PlayerEvents[K]): void;

    lowLatency: boolean;
    abrEnabled: boolean;
    variantId: number;
    nextVariantId: number | null;
    tracks: Track[];
    variants: Variant[];
}
"#;

#[wasm_bindgen(inspectable, skip_typescript)]
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
    pub fn set_low_latency(&mut self, low_latency: bool) {
        self.inner.set_low_latency(low_latency);
    }

    #[wasm_bindgen(getter = lowLatency)]
    pub fn low_latency(&self) -> bool {
        self.inner.low_latency()
    }

    pub fn load(&mut self, url: &str) -> Result<(), JsValue> {
        self.inner.set_url(url);

        if self.inner.video_element().is_none() {
            return Ok(());
        }

        self.shutdown();
        self.spawn_runner();

        Ok(())
    }

    #[wasm_bindgen(getter = tracks)]
    pub fn tracks(&self) -> JsValue {
        self.inner
            .tracks()
            .iter()
            .map(JsValue::from_serde)
            .collect::<serde_json::Result<js_sys::Array>>()
            .unwrap()
            .unchecked_into()
    }

    #[wasm_bindgen(getter = variants)]
    pub fn variants(&self) -> JsValue {
        self.inner
            .variants()
            .iter()
            .map(JsValue::from_serde)
            .collect::<serde_json::Result<js_sys::Array>>()
            .unwrap()
            .unchecked_into()
    }

    #[wasm_bindgen(js_name = on)]
    pub fn on(&mut self, event: &str, f: JsValue) {
        self.inner.add_event_listener(event, f, false);
    }

    #[wasm_bindgen(js_name = removeListener)]
    pub fn remove_listener(&mut self, event: &str, f: JsValue) {
        self.inner.remove_event_listener(event, f);
    }

    #[wasm_bindgen(js_name = off)]
    pub fn off(&mut self, event: &str, f: JsValue) {
        self.inner.remove_event_listener(event, f);
    }

    #[wasm_bindgen(js_name = once)]
    pub fn once(&mut self, event: &str, f: JsValue) {
        self.inner.add_event_listener(event, f, true);
    }

    #[wasm_bindgen(setter = abrEnabled)]
    pub fn set_abr_enabled(&mut self, abr_enabled: bool) {
        self.inner.set_abr_enabled(abr_enabled);
    }

    #[wasm_bindgen(getter = abrEnabled)]
    pub fn abr_enabled(&self) -> bool {
        self.inner.abr_enabled()
    }

    pub fn attach(&mut self, el: HtmlVideoElement) -> Result<(), JsValue> {
        let Ok(element) = el.dyn_into::<web_sys::HtmlVideoElement>() else {
            return Err(JsValue::from_str("element is not a video element"));
        };

        if let Some(el) = self.inner.video_element() {
            if el.is_same_node(Some(&element)) {
                return Err(JsValue::from_str("element is already attached"));
            }
        }

        self.inner.set_video_element(Some(element));

        if self.inner.url().is_empty() {
            return Ok(());
        }

        self.shutdown();
        self.spawn_runner();

        Ok(())
    }

    pub fn detach(&mut self) {
        self.shutdown();
        self.inner.set_video_element(None);
    }

    pub fn shutdown(&self) {
        self.shutdown_sender.send(()).ok();
    }

    /// Gracefully switch to this variant id when the current segment is finished.
    #[wasm_bindgen(setter = nextVariantId)]
    pub fn set_next_variant_id(&mut self, track_id: Option<u32>) {
        if let Some(track_id) = track_id {
            self.inner.set_abr_enabled(false);
            if self.inner.variants().len() as u32 <= track_id {
                return;
            }
        }

        self.inner
            .set_next_variant_id(track_id.map(NextVariant::Switch))
    }

    /// Get the variant track id that will be switched to.
    #[wasm_bindgen(getter = nextVariantId)]
    pub fn next_variant_id(&self) -> Option<u32> {
        match self.inner.next_variant_id() {
            Some(NextVariant::Switch(track_id)) | Some(NextVariant::Force(track_id)) => {
                Some(track_id)
            }
            None => None,
        }
    }

    /// Get the variant id that is currently active.
    #[wasm_bindgen(getter = variantId)]
    pub fn variant_id(&self) -> u32 {
        self.inner.active_variant_id()
    }

    /// Force switch to this variant id immediately.
    #[wasm_bindgen(setter = variantId)]
    pub fn set_variant_id(&mut self, track_id: u32) {
        if self.inner.variants().len() as u32 > track_id {
            self.inner.set_abr_enabled(false);
            self.inner
                .set_next_variant_id(Some(NextVariant::Force(track_id)));
        }
    }
}

impl Player {
    fn spawn_runner(&self) {
        spawn_local(
            PlayerRunner::new(self.inner.clone(), self.shutdown_sender.subscribe()).start(),
        );
    }
}
