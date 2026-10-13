//! Hosts `greybel-lsp` in a browser worker. Startup accepts the host's worker scope and serves the language-server
//! protocol on its port, carrying one JSON-RPC message per protocol message.

mod language_server;
pub mod protocol;

use duplex_driver_web::WorkerScope;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::language_server::LanguageServer;

/// Installs panic reporting and serves the worker host until client exit. Panics are logged and close the channel.
#[wasm_bindgen(start, private)]
fn serve() {
    WorkerScope::report_panics();

    let scope = WorkerScope::accept();

    wasm_bindgen_futures::spawn_local(async move {
        LanguageServer::serve(scope.await.channel());
    });
}
