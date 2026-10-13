//! Hosts the builder in a browser worker. Module startup accepts the host's worker scope and serves build requests
//! on its transport.

use duplex_driver_web::WorkerScope;
use greybel_builder::Builder;
use wasm_bindgen::prelude::wasm_bindgen;

/// Installs panic reporting and handles build requests on the host's worker transport until the builder protocol
/// closes.
#[wasm_bindgen(start, private)]
fn serve() {
    WorkerScope::report_panics();

    let scope = WorkerScope::accept();

    wasm_bindgen_futures::spawn_local(async move {
        let scope = scope.await;

        Builder::new(scope.transport()).serve().await;
    });
}
