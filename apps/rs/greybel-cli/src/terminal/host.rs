use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

use greybel_documents::{PathResolution, Uri};
use greybel_documents_fs_provider::FsProvider;
use greybel_io::{BufferedStdout, Stdout};
use greybel_io_terminal::TerminalStdin;
use greybel_runtime::protocol::{LogMessage, StopNotice};
use greybel_runtime::{Controller, Host, Io, Runtime, WorldFuture};
use greybel_runtime_world::WorldSource;
use greyhack_terminal_text::LineSplitRule;
use scheduler::{Clock, TokioScheduler};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use super::screen::RichTextScreen;
use crate::report;

/// Process-backed runtime host providing filesystem and memory documents, terminal I/O, and debugger stop delivery.
pub struct TerminalHost {
    memory: Arc<BTreeMap<Uri, String>>,
    stops: UnboundedSender<StopNotice>,
    /// Clock and timer scheduler for programs and generated worlds.
    scheduler: TokioScheduler,
    world: Rc<WorldSource>,
    split_rule: LineSplitRule,
}

impl TerminalHost {
    /// Starts a terminal runtime on the current `LocalSet` and returns its controller and debugger stop receiver.
    /// Programs read `memory` before the filesystem, find their world through `world`, and split long output lines by
    /// `split_rule`.
    pub fn start(
        memory: Arc<BTreeMap<Uri, String>>,
        world: WorldSource,
        split_rule: LineSplitRule,
    ) -> (Controller<Self>, UnboundedReceiver<StopNotice>) {
        let (stops, receiver) = mpsc::unbounded_channel();
        let (runtime, controller) = Runtime::with_host(Self {
            memory,
            stops,
            scheduler: TokioScheduler::new(),
            world: Rc::new(world),
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

    fn world(&self, seed: &str) -> WorldFuture {
        let (source, seed, clock) = (Rc::clone(&self.world), seed.to_owned(), self.clock());

        Box::pin(async move {
            source
                .launch_world(&seed, clock, |failure| {
                    eprintln!("The program runs in a world of its own: {failure}.");
                })
                .await
        })
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
