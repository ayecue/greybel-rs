//! In-process runtime host with in-memory documents, captured I/O, and stop and logpoint sinks.

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use greybel_documents::PathResolution;
use greybel_documents_memory_provider::MemoryProvider;
use greybel_interpreter::Clock;
use greybel_runtime::protocol::{Environment, Launch, LaunchFailure, LogMessage, StopNotice};
use greybel_runtime::{Host, Io, LaunchWorld, WorldFuture};
use greybel_runtime_world::LocalWorld;
use greybel_test_support::io::{CapturedStdout, PrintLog, ScriptedStdin};
use greyscript_intrinsics::ServerConnection;
use scheduler::TokioScheduler;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::task::LocalSet;

pub const MAIN: &str = "/main.src";

/// Host serving in-memory documents, scripted input, captured output, and stop and logpoint events.
pub struct MemoryHost {
    files: Vec<(&'static str, &'static str)>,
    printed: PrintLog,
    input: Vec<&'static str>,
    stops: UnboundedSender<StopNotice>,
    logs: Rc<RefCell<Vec<String>>>,
    /// Generates a world for each simulated launch; without, such launches are unavailable.
    pub has_worlds: bool,
    /// World connections of the launches.
    connections: Rc<RefCell<Vec<ServerConnection>>>,
}

impl Host for MemoryHost {
    type Provider = MemoryProvider;

    fn provider(&self, resolution: PathResolution) -> MemoryProvider {
        let mut provider = MemoryProvider::with_resolution(resolution);

        for (path, content) in &self.files {
            provider.insert(path, *content);
        }

        provider
    }

    fn io(&self) -> Io {
        Io {
            stdout: Rc::new(CapturedStdout(Rc::clone(&self.printed))),
            stdin: Rc::new(ScriptedStdin::new(&self.input)),
        }
    }

    fn clock(&self) -> Rc<dyn Clock> {
        Rc::new(TokioScheduler::new())
    }

    fn world(&self, seed: &str) -> WorldFuture {
        let (seed, clock, has_worlds) = (seed.to_owned(), self.clock(), self.has_worlds);
        let connections = Rc::clone(&self.connections);

        Box::pin(async move {
            if !has_worlds {
                return Err(LaunchFailure::Unavailable);
            }

            let world = LocalWorld::generate(&seed, clock).await;
            let connection = world.connect_on_tokio();

            connections.borrow_mut().push(connection.clone());

            Ok(LaunchWorld {
                connection,
                guard: Some(Box::new(world)),
            })
        })
    }

    fn report_stop(&self, notice: &StopNotice) {
        let _ = self.stops.send(notice.clone());
    }

    fn report_log(&self, message: &LogMessage<'_>) {
        self.logs.borrow_mut().push(message.text.to_string());
    }
}

/// Memory host plus handles to its captured output, stops, and logpoint messages.
pub struct Setup {
    pub host: MemoryHost,
    pub printed: PrintLog,
    pub stops: UnboundedReceiver<StopNotice>,
    pub logs: Rc<RefCell<Vec<String>>>,
    pub connections: Rc<RefCell<Vec<ServerConnection>>>,
}

/// Creates a memory host serving `code` as [`MAIN`] with scripted `input`.
pub fn setup(code: &'static str, input: Vec<&'static str>) -> Setup {
    let (sender, stops) = mpsc::unbounded_channel();
    let printed = PrintLog::default();
    let logs = Rc::default();
    let connections = Rc::<RefCell<Vec<ServerConnection>>>::default();

    Setup {
        host: MemoryHost {
            files: vec![(MAIN, code)],
            printed: Rc::clone(&printed),
            input,
            stops: sender,
            logs: Rc::clone(&logs),
            has_worlds: true,
            connections: Rc::clone(&connections),
        },
        printed,
        stops,
        logs,
        connections,
    }
}

/// Creates a mock-world launch for [`MAIN`] with the requested debug flag and parameters.
pub fn launch(debug: bool, params: &[&str]) -> Launch<'static> {
    Launch {
        entry: MemoryProvider::uri(MAIN).to_string().into(),
        root: None,
        extensions: Cow::Owned(vec!["src".to_owned()]),
        environment: Environment::Mock {
            seed: "test".into(),
            memory_limit: 1024,
            user: None,
        },
        debug,
        stop_on_entry: false,
        params: params
            .iter()
            .map(|param| (*param).to_owned())
            .collect::<Vec<_>>()
            .into(),
        variables: BTreeMap::new(),
        strict_mode: false,
        program_name: "myprogram".into(),
        disabled_syntax: None,
        multiline_strings: None,
    }
}

/// Runs `future` on a current-thread Tokio runtime and `LocalSet`.
pub fn run<F: Future>(future: F) -> F::Output {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("Tokio's runtime builds");

    LocalSet::new().block_on(&runtime, future)
}
