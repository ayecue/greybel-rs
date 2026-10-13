//! Builder request routing and the sequential build/import service loop.

mod router;
mod state;
mod task;

use std::pin::pin;
use std::rc::Rc;

use duplex_core::{Driver, Transport};
use futures_util::future::{Either, select};
use router::BuilderHandler;
pub(crate) use state::BuilderState;
use task::{Settle as _, Task};
use tokio::sync::mpsc::{self, UnboundedReceiver};

use crate::protocol::BuilderRouter as _;

/// Serves one duplex editor and processes its builds and imports sequentially.
pub struct Builder<D: Driver> {
    state: Rc<BuilderState<D>>,
    tasks: UnboundedReceiver<Task>,
}

impl<D: Driver> Builder<D> {
    /// Creates a builder provider on `transport` with builder and documents channels.
    #[must_use]
    pub fn new(transport: &Transport<D>) -> Self {
        let (sender, tasks) = mpsc::unbounded_channel();
        let state = Rc::new(BuilderState::new(transport));

        BuilderHandler(sender).attach(&state.builder);

        Self { state, tasks }
    }

    /// Processes queued builds and imports until the builder protocol or task queue closes.
    pub async fn serve(mut self) {
        let mut close_event = pin!(self.state.builder.close_event());

        loop {
            let task = {
                let next = pin!(self.tasks.recv());

                match select(next, close_event.as_mut()).await {
                    Either::Left((Some(task), _)) => task,
                    Either::Left((None, _)) | Either::Right(_) => return,
                }
            };

            match task {
                Task::Build(build, responder) => {
                    responder.settle(
                        crate::build(&build, |resolution| self.state.provider(resolution)).await,
                    );
                }
                Task::Import(import, responder) => {
                    responder.settle(
                        crate::import(&import, |resolution| self.state.provider(resolution)).await,
                    );
                }
            }
        }
    }
}
