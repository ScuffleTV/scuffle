use wasm_bindgen::prelude::*;

mod hls;
mod player;
mod tracing_wasm;

#[wasm_bindgen(start, skip_typescript)]
pub fn main() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    tracing_wasm::set_as_global_default();

    Ok(())
}
