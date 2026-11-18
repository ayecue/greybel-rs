//! Serves one builder session over stdin/stdout, for the TypeScript `greybel-builder` package's tests: what the app
//! `greybel-builder-stdio` serves.

use duplex_driver_stdio::StreamDriver;
use greybel_builder::Builder;
use tokio::task::LocalSet;

fn main() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Tokio's runtime builds");

    LocalSet::new().block_on(&runtime, async {
        let transport = StreamDriver::stdio();

        Builder::new(&transport).serve().await;
    });
}
