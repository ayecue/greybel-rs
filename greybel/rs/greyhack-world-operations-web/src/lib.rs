//! Hosts a world server in a browser worker. The worker URL's `seed` parameter selects the world seed,
//! defaulting to `test`. Every host connection is attached, including the initial scope.

use std::rc::Rc;

use duplex_driver_web::WorkerScope;
use futures_util::StreamExt;
use greyhack_world::{PlayerOptions, World};
use greyhack_world_operations::WorldServer;
use scheduler::WebScheduler;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::wasm_bindgen;
use web_sys::{Url, WorkerGlobalScope};

/// Fallback seed when the worker URL omits `seed`.
const DEFAULT_SEED: &str = "test";

#[wasm_bindgen(start, private)]
fn serve() {
    WorkerScope::report_panics();

    let mut connections = WorkerScope::listen();

    wasm_bindgen_futures::spawn_local(async move {
        let world = World::from_seed(&seed(), &PlayerOptions::default())
            .await
            .expect("the world generates");
        let server = WorldServer::new(world, Rc::new(WebScheduler));

        while let Some(scope) = connections.next().await {
            server.attach(scope.transport());
        }
    });
}

/// Reads `seed` from the worker script URL, falling back to `DEFAULT_SEED`.
fn seed() -> String {
    js_sys::global()
        .dyn_into::<WorkerGlobalScope>()
        .ok()
        .and_then(|scope| Url::new(&scope.location().href()).ok())
        .and_then(|url| url.search_params().get("seed"))
        .unwrap_or_else(|| DEFAULT_SEED.to_owned())
}
