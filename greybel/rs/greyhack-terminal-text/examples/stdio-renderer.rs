//! Serves the terminal renderer over stdin/stdout, for the TypeScript `greyhack-terminal-text` package's tests: the
//! renderer `greybel-runtime-stdio` serves beside its runtime.

use duplex_driver_stdio::StreamDriver;
use greyhack_terminal_text::Renderer;
use tokio::task::LocalSet;

fn main() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Tokio's runtime builds");

    LocalSet::new().block_on(&runtime, async {
        let transport = StreamDriver::stdio();

        Renderer::new(&transport).serve().await;
    });
}
