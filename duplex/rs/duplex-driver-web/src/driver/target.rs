use wasm_bindgen::prelude::*;
use web_sys::EventTarget;

#[wasm_bindgen]
extern "C" {
    /// JavaScript worker, global scope, or `MessagePort` event target for transport batches.
    #[wasm_bindgen(extends = EventTarget)]
    #[derive(Clone)]
    pub type MessageTarget;

    #[wasm_bindgen(method, structural, js_name = postMessage)]
    pub(crate) fn post_message_with_transfer(
        this: &MessageTarget,
        message: &JsValue,
        transfer: &JsValue,
    );
}
