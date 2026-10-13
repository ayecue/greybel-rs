use std::time::Duration;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = setInterval)]
    fn set_interval(handler: &JsValue, timeout: i32) -> JsValue;

    #[wasm_bindgen(js_name = clearInterval)]
    fn clear_interval(id: &JsValue);

    #[wasm_bindgen(js_name = setTimeout)]
    fn set_timeout(handler: &JsValue, timeout: i32) -> JsValue;

    #[wasm_bindgen(js_name = clearTimeout)]
    fn clear_timeout(id: &JsValue);

    #[wasm_bindgen(js_namespace = performance, js_name = now)]
    fn performance_now() -> f64;
}

/// JavaScript `setTimeout` or `setInterval` callback handle. Drop clears the timer.
pub struct WebTimer {
    id: JsValue,
    repeats: bool,
    _handler: Closure<dyn FnMut()>,
}

impl WebTimer {
    pub(super) fn interval(period: Duration, handler: impl FnMut() + 'static) -> Self {
        let handler = Closure::<dyn FnMut()>::new(handler);

        Self {
            id: set_interval(handler.as_ref(), Self::millis(period)),
            repeats: true,
            _handler: handler,
        }
    }

    pub(super) fn once(delay: Duration, handler: Box<dyn FnOnce()>) -> Self {
        let mut handler = Some(handler);
        let handler = Closure::<dyn FnMut()>::new(move || {
            if let Some(handler) = handler.take() {
                handler();
            }
        });

        Self {
            id: set_timeout(handler.as_ref(), Self::millis(delay)),
            repeats: false,
            _handler: handler,
        }
    }

    pub(super) fn now() -> Duration {
        Duration::from_secs_f64(performance_now() / 1000.0)
    }

    fn millis(duration: Duration) -> i32 {
        i32::try_from(duration.as_millis()).unwrap_or(i32::MAX)
    }
}

impl Drop for WebTimer {
    fn drop(&mut self) {
        if self.repeats {
            clear_interval(&self.id);
        } else {
            clear_timeout(&self.id);
        }
    }
}
