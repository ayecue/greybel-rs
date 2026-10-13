//! Hosts API reference documentation in a browser worker. Module startup accepts the host's worker scope and serves
//! requests on its transport.

use duplex_driver_web::WorkerScope;
use greybel_api_reference::Documentation;
use wasm_bindgen::prelude::wasm_bindgen;

/// Installs panic reporting and handles reference requests on the host's worker transport until the protocol
/// closes.
#[wasm_bindgen(start, private)]
fn serve() {
    WorkerScope::report_panics();

    let scope = WorkerScope::accept();

    wasm_bindgen_futures::spawn_local(async move {
        let scope = scope.await;

        Documentation::new(scope.transport()).serve().await;
    });
}
