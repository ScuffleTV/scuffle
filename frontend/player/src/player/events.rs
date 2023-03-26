use serde::Serialize;
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use super::track::Track;

#[wasm_bindgen(getter_with_clone)]
pub struct EventError {
    #[wasm_bindgen(readonly)]
    pub error: JsValue,
}

impl From<JsValue> for EventError {
    fn from(error: JsValue) -> Self {
        Self { error }
    }
}

#[wasm_bindgen(typescript_custom_section)]
const _: &'static str = r#"
type OnErrorFunction = (this: null, evt: EventError) => void;
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "OnErrorFunction")]
    pub type OnErrorFunction;

    #[wasm_bindgen(catch, method, js_name = call)]
    pub fn call(this: &OnErrorFunction, ctx: JsValue, evt: EventError) -> Result<(), JsValue>;
}

#[wasm_bindgen(typescript_custom_section)]
const _: &'static str = r#"
type OnManifestLoadedFunction = (this: null, evt: EventManifestLoaded) => void;
"#;

#[derive(Tsify, Serialize)]
#[tsify(into_wasm_abi)]
pub struct EventManifestLoaded {
    pub is_master_playlist: bool,
    pub tracks: Vec<Track>,
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "OnManifestLoadedFunction")]
    pub type OnManifestLoadedFunction;

    #[wasm_bindgen(catch, method, js_name = call)]
    pub fn call(
        this: &OnManifestLoadedFunction,
        ctx: JsValue,
        evt: EventManifestLoaded,
    ) -> Result<(), JsValue>;
}
