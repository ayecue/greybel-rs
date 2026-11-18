//! Serves one editor runtime over stdin/stdout, for the TypeScript `greybel-runtime` package's tests: what the app
//! `greybel-runtime-stdio` serves, without its terminal renderer, and with a world generated for each launch instead of a
//! shared one.

use std::rc::Rc;

use duplex_driver_stdio::StreamDriver;
use greybel_runtime::Runtime;
use greybel_runtime_world::LocalWorldFactory;
use tokio::task::LocalSet;

fn main() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Tokio's runtime builds");

    LocalSet::new().block_on(&runtime, async {
        let transport = StreamDriver::stdio();

        Runtime::with_world_factory(&transport, Rc::new(LocalWorldFactory))
            .serve()
            .await;
    });
}
