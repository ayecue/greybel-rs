use duplex_driver_web::WorkerScope;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::protocol::io;
use crate::protocol::terminal::View;
use crate::service::TerminalService;

/// Installs panic reporting and serves the terminal on the worker port. It displays relayed output until the page
/// closes the view, then drops the io channel.
#[wasm_bindgen(start, private)]
fn serve() {
    WorkerScope::report_panics();

    let scope = WorkerScope::accept();

    wasm_bindgen_futures::spawn_local(async move {
        let scope = scope.await;
        let view = scope.channel::<View>();
        let page = scope.channel::<io::Page>();

        TerminalService::serve(&view, &page);
        view.close_event().await;
        drop(page);
    });
}
