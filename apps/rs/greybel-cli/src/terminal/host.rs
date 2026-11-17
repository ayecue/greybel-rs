use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

use greybel_documents::{PathResolution, Uri};
use greybel_documents_fs_provider::FsProvider;
use greybel_io::{BufferedStdout, Stdout};
use greybel_io_terminal::TerminalStdin;
use greybel_runtime::protocol::{LogMessage, StopNotice};
use greybel_runtime::{Controller, Host, Io, Runtime, connect_on_tokio};
use greyhack_world_operations::WorldServer;
use greyscript_intrinsics::ServerConnection;
use scheduler::{Clock, TokioScheduler};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use greyhack_terminal_text::LineSplitRule;

use super::screen::RichTextScreen;
use crate::report;

/// Process-backed runtime host providing filesystem and memory documents, terminal I/O, and debugger stop delivery.
pub struct TerminalHost {
    memory: Arc<BTreeMap<Uri, String>>,
    stops: UnboundedSender<StopNotice>,
    /// Clock and timer scheduler for programs and generated worlds.
    scheduler: TokioScheduler,
    /// Shared game-server world; programs use individual generated worlds when absent.
    world: Option<ServerConnection>,
    split_rule: LineSplitRule,
}

impl TerminalHost {
    /// Starts a terminal runtime on the current `LocalSet`. Programs read `memory` before the filesystem
    /// and use `world` when provided; their output splits long lines by `split_rule`. Returns the runtime controller
    /// and debugger stop receiver.
    pub fn start(
        memory: Arc<BTreeMap<Uri, String>>,
        world: Option<ServerConnection>,
        split_rule: LineSplitRule,
    ) -> (Controller<Self>, UnboundedReceiver<StopNotice>) {
        let (stops, receiver) = mpsc::unbounded_channel();
        let (runtime, controller) = Runtime::with_host(Self {
            memory,
            stops,
            scheduler: TokioScheduler::new(),
            world,
            split_rule,
        });

        tokio::task::spawn_local(runtime.serve());

        (controller, receiver)
    }
}

impl Host for TerminalHost {
    type Provider = FsProvider;

    fn provider(&self, resolution: PathResolution) -> FsProvider {
        FsProvider::new(resolution).with_memory(Arc::clone(&self.memory))
    }

    fn io(&self) -> Io {
        let stdout: Rc<dyn Stdout> = Rc::new(BufferedStdout::new(
            RichTextScreen::new(self.split_rule),
            self.clock(),
        ));
        let stdin = Rc::new(TerminalStdin::new(Rc::clone(&stdout)));

        Io { stdout, stdin }
    }

    fn clock(&self) -> Rc<dyn Clock> {
        Rc::new(self.scheduler.clone())
    }

    fn world(&self) -> Option<ServerConnection> {
        self.world.clone()
    }

    fn connect(&self, server: &WorldServer) -> ServerConnection {
        connect_on_tokio(server)
    }

    fn report_stop(&self, notice: &StopNotice) {
        let _ = self.stops.send(notice.clone());
    }

    fn report_log(&self, message: &LogMessage<'_>) {
        match &message.location {
            Some(location) => eprintln!(
                "{}: {}",
                report::place(&location.uri, location.line, location.character),
                message.text
            ),
            None => eprintln!("{}", message.text),
        }
    }
}
