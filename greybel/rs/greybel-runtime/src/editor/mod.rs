//! Duplex editor-backed runtime host. Uses the runtime protocol for control, io for program I/O, and documents
//! for file access.

mod router;

use std::future::Future;
use std::rc::Rc;

use duplex_core::{Channel, ChannelOptions, Driver, Transport};
use greybel_documents::PathResolution;
use greybel_documents_host_provider::HostProvider;
use greybel_documents_host_provider::protocol::Server;
use greybel_interpreter::{Clock, Stdout};
use greybel_io_web::protocol::Program;
use greybel_io_web::{PageScreen, PageStdin, PageStdout};
use greyhack_world_operations::WorldServer;
use greyscript_intrinsics::ServerConnection;
use router::RunnerHandler;

use crate::protocol::{LogMessage, Runner, RunnerRouter as _, StopNotice};
use crate::runtime::{Host, Io, Runtime};

/// Runtime host backed by the remote editor. Supplies editor documents, page stdin/stdout, and stop/logpoint
/// notifications.
pub struct EditorHost<D: Driver> {
    runner: Channel<Runner, D>,
    program: Channel<Program, D>,
    server: Channel<Server, D>,
    world: Option<ServerConnection>,
}

impl<D: Driver> Runtime<EditorHost<D>> {
    /// Creates an editor runtime on `transport` with runtime, io, and documents channels. Runs until the editor closes
    /// the runtime protocol and gives each program a separate from_seed world.
    #[must_use]
    pub fn new(transport: &Transport<D>) -> Self {
        Self::of(transport, None)
    }

    /// Creates an editor runtime that runs all programs in the shared world from `world`.
    #[must_use]
    pub fn with_world(transport: &Transport<D>, world: ServerConnection) -> Self {
        Self::of(transport, Some(world))
    }

    fn of(transport: &Transport<D>, world: Option<ServerConnection>) -> Self {
        let host = EditorHost {
            runner: Channel::new(transport, ChannelOptions::default()),
            program: Channel::new(transport, ChannelOptions::default()),
            server: Channel::new(transport, ChannelOptions::default()),
            world,
        };
        let runner = host.runner.clone();
        let (runtime, controller) = Self::with_host(host);

        RunnerHandler(controller).attach(&runner);

        runtime
    }
}

impl<D: Driver> Host for EditorHost<D> {
    type Provider = HostProvider<D>;

    fn provider(&self, resolution: PathResolution) -> HostProvider<D> {
        HostProvider::new(self.server.clone(), resolution)
    }

    fn io(&self) -> Io {
        let stdout: Rc<dyn Stdout> = Rc::new(PageStdout::new(
            PageScreen::new(&self.program),
            self.program.driver().scheduler().clone(),
        ));
        let stdin = Rc::new(PageStdin::new(&self.program, Rc::clone(&stdout)));

        Io { stdout, stdin }
    }

    /// Uses the editor transport scheduler as the program clock in browser workers and native processes.
    fn clock(&self) -> Rc<dyn Clock> {
        Rc::new(self.runner.driver().scheduler().clone())
    }

    fn world(&self) -> Option<ServerConnection> {
        self.world.clone()
    }

    fn connect(&self, server: &WorldServer) -> ServerConnection {
        ServerConnection::new(Channel::new(
            &server.connect(self.runner.driver().scheduler().clone()),
            ChannelOptions::default(),
        ))
    }

    fn report_stop(&self, notice: &StopNotice) {
        let _ = self.runner.send(notice);
    }

    fn report_log(&self, message: &LogMessage<'_>) {
        let _ = self.runner.send(message);
    }

    fn close_event(&self) -> impl Future<Output = ()> + 'static {
        let close_event = self.runner.close_event();

        async move {
            close_event.await;
        }
    }
}
