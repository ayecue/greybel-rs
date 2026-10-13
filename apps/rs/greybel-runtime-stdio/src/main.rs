//! Serve one editor runtime over stdin/stdout using the editor terminal renderer. Exit when the editor closes the
//! runtime protocol or its input. Use `--world <address>` to connect programs to a shared TCP world server.

use std::{fmt, io};

use duplex_driver_stdio::StreamDriver;
use greybel_runtime::{Runtime, connect_over_tcp};
use greyhack_terminal_text::Renderer;
use tokio::task::LocalSet;

/// Runtime startup error.
#[derive(thiserror::Error)]
enum RuntimeError {
    #[error("{0}")]
    Arguments(#[from] WorldAddressError),
    #[error("the game server at {address} is unreachable: {source}")]
    World {
        address: String,
        #[source]
        source: io::Error,
    },
}

/// Matches the debug representation of the `String` error this binary returned before.
impl fmt::Debug for RuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.to_string(), formatter)
    }
}

/// Invalid world command-line address error.
#[derive(Debug, thiserror::Error)]
enum WorldAddressError {
    #[error("usage: greybel-runtime [--world <address>]")]
    Usage,
}

fn main() -> Result<(), RuntimeError> {
    let world = world_address()?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Tokio's runtime builds");

    LocalSet::new().block_on(&runtime, async {
        let transport = StreamDriver::stdio();

        tokio::task::spawn_local(Renderer::new(&transport).serve());

        let runtime = match world {
            Some(address) => {
                let world =
                    connect_over_tcp(&address)
                        .await
                        .map_err(|source| RuntimeError::World {
                            address: address.clone(),
                            source,
                        })?;

                Runtime::with_world(&transport, world)
            }
            None => Runtime::new(&transport),
        };

        runtime.serve().await;

        Ok(())
    })
}

/// Parse the optional `--world <address>` argument and return a usage error for other or missing arguments.
fn world_address() -> Result<Option<String>, WorldAddressError> {
    let mut arguments = std::env::args().skip(1);

    match (arguments.next().as_deref(), arguments.next()) {
        (None, _) => Ok(None),
        (Some("--world"), Some(address)) => Ok(Some(address)),
        _ => Err(WorldAddressError::Usage),
    }
}
