use std::future::{Future, pending};
use std::rc::Rc;

use greybel_documents::{AsyncResourceProvider, PathResolution};
use greybel_interpreter::{Clock, Stdin, Stdout};
use greyhack_world_operations::WorldServer;
use greyscript_intrinsics::ServerConnection;

use crate::protocol::{LogMessage, StopNotice};

/// Runtime resources and events: documents, standard I/O, clock, stop/logpoint reports, and world connections.
/// [`crate::EditorHost`] implements this interface for duplex editors.
pub trait Host: 'static {
    /// Asynchronous document provider for programs.
    type Provider: AsyncResourceProvider + 'static;

    /// Creates a document provider that resolves imports with `resolution`.
    fn provider(&self, resolution: PathResolution) -> Self::Provider;

    /// Creates stdin/stdout for the next program.
    fn io(&self) -> Io;

    /// Clock for `wait`, `time`, cooldowns, and isolated worlds. Waits must yield to process pause and termination.
    fn clock(&self) -> Rc<dyn Clock>;

    /// Reports a program stop at `notice`'s location.
    fn report_stop(&self, notice: &StopNotice);

    /// Reports an emitted logpoint.
    fn report_log(&self, message: &LogMessage<'_>);

    /// Returns a shared world-server connection; `None` creates a separate from_seed world per program.
    fn world(&self) -> Option<ServerConnection> {
        None
    }

    /// Connects a program to in-process world `server`.
    fn connect(&self, server: &WorldServer) -> ServerConnection;

    /// Completes on host-initiated runtime shutdown. The default never completes, so shutdown otherwise requires all
    /// [`crate::Controller`] handles to be dropped.
    fn close_event(&self) -> impl Future<Output = ()> + 'static {
        pending()
    }
}

/// Program stdout and stdin.
pub struct Io {
    pub stdout: Rc<dyn Stdout>,
    pub stdin: Rc<dyn Stdin>,
}
