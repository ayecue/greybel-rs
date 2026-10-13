//! Serves one editor runtime over stdin/stdout, for the TypeScript `greybel-runtime` package's tests: what the app
//! `greybel-runtime-stdio` serves, without its terminal renderer and shared world.

use duplex_driver_stdio::StreamDriver;
use greybel_runtime::Runtime;
use tokio::task::LocalSet;

fn main() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Tokio's runtime builds");

    LocalSet::new().block_on(&runtime, async {
        let transport = StreamDriver::stdio();

        Runtime::new(&transport).serve().await;
    });
}
