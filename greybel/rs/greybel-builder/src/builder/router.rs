use duplex_core::{NextMessage, Responder};
use tokio::sync::mpsc::UnboundedSender;

use super::task::{Settle as _, Task};
use crate::protocol::{Build, BuildFailure, BuildResult, BuilderRouter, Import, Transform};

/// Queues build and import tasks in editor order; handles transformations synchronously.
pub(super) struct BuilderHandler(pub(super) UnboundedSender<Task>);

impl BuilderHandler {
    /// Queues `task` for the service loop. A terminated loop drops the responder, closing the editor's response channel.
    fn queue(&self, task: Task) {
        let _ = self.0.send(task);
    }
}

impl BuilderRouter for BuilderHandler {
    fn on_build(
        &self,
        message: Build<'static>,
        responder: Responder<BuildResult, BuildFailure>,
        _next: NextMessage<Build<'static>>,
    ) {
        self.queue(Task::Build(message, responder));
    }

    fn on_import(
        &self,
        message: Import<'static>,
        responder: Responder<u32, BuildFailure>,
        _next: NextMessage<Import<'static>>,
    ) {
        self.queue(Task::Import(message, responder));
    }

    fn on_transform(
        &self,
        message: Transform<'static>,
        responder: Responder<String, BuildFailure>,
        _next: NextMessage<Transform<'static>>,
    ) {
        responder.settle(crate::transform(&message));
    }
}
