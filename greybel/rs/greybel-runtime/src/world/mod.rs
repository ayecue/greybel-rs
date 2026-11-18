//! Worlds that simulated launches run in.

#[cfg(not(target_family = "wasm"))]
pub(crate) mod machine;
#[cfg(not(target_family = "wasm"))]
pub(crate) mod tcp;

use std::any::Any;
use std::rc::Rc;

use futures_util::future::LocalBoxFuture;
use greybel_interpreter::Clock;
use greyscript_intrinsics::ServerConnection;

use crate::protocol::LaunchFailure;

/// World of a launch that [`crate::Host::world`] returns, or the failure that refuses the launch.
pub type WorldFuture = LocalBoxFuture<'static, Result<LaunchWorld, LaunchFailure>>;

pub struct LaunchWorld {
    pub connection: ServerConnection,
    /// Value the launch retains until it ends, such as an in-process world server or a connection that closes when
    /// dropped.
    pub guard: Option<Box<dyn Any>>,
}

impl LaunchWorld {
    /// World of `connection` to a server that runs on its own, such as a shared world server.
    #[must_use]
    pub fn new(connection: ServerConnection) -> Self {
        Self {
            connection,
            guard: None,
        }
    }
}

/// Creates a launch world from its seed using scheduler `S`. Greybel's in-process implementation is
/// `greybel_runtime_world::LocalWorldFactory`.
pub trait WorldFactory<S> {
    fn create(
        &self,
        seed: &str,
        clock: Rc<dyn Clock>,
        scheduler: S,
    ) -> LocalBoxFuture<'static, LaunchWorld>;
}
