//! Hosts the editor runtime and terminal renderer in a browser worker on the supplied port. Programs can use an
//! optional world-protocol port to share a `greyhack-world-operations-web` world server.

use std::rc::Rc;

use duplex_core::ChannelOptions;
use duplex_driver_web::{EventLoop, WorkerScope};
use greybel_runtime::Runtime;
use greyhack_terminal_text::Renderer;
use greyscript_intrinsics::ServerConnection;
use greyscript_intrinsics::protocol::World;
use wasm_bindgen::prelude::wasm_bindgen;

/// Installs panic reporting, starts the renderer and runtime, and serves until host shutdown. The runtime yields
/// to the worker event loop to process host messages, including pause.
#[wasm_bindgen(start, private)]
fn serve() {
    WorkerScope::report_panics();

    let scope = WorkerScope::accept();

    wasm_bindgen_futures::spawn_local(async move {
        let scope = scope.await;

        wasm_bindgen_futures::spawn_local(Renderer::new(scope.transport()).serve());

        let world = scope.port_transport_of::<World>().map(|transport| {
            ServerConnection::new(duplex_core::Channel::new(
                transport,
                ChannelOptions::default(),
            ))
        });
        let runtime = match world {
            Some(world) => Runtime::with_world(scope.transport(), world),
            None => Runtime::new(scope.transport()),
        };

        runtime
            .with_yielder(Rc::new(|| Box::pin(EventLoop::yield_now())))
            .serve()
            .await;
    });
}
