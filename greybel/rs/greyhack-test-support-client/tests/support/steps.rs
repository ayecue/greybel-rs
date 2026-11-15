//! Scripts that pause at each input they wait for, while both worlds make the same change, before the input is typed.

use std::rc::Rc;

use greybel_interpreter::{InputFuture, InterpreterOptions, Key, RuntimeErrorKind, Stdin, Stdout};
use greybel_test_support::greyhack::run_in_at;
use greybel_test_support::interpreter::constant_clock;
use greybel_test_support::io::MarkedStdout;
use greyhack_test_support_client::protocol::test_support::NetworkSnapshot;
use greyhack_test_support_client::{ExecutionStep, TestClient};
use greyhack_world::{DatabaseOpen, World};
use greyhack_world_core::Database;
use greyhack_world_operations::WorldServer;
use tokio::sync::{mpsc, oneshot};

use super::intrinsics::{PROGRAM_NAME, snapshot_world};
use super::parity::{Outcome, game_error, without_ack_counts};

/// What both worlds change while the script waits for an input.
pub enum WorldChange {
    /// The clock moves this many seconds forward, running what is due on the way.
    AdvanceClock(u64),
    /// The router or switch at `local_ip` of the player's network gets a kernel that isn't a library, so it no longer
    /// starts.
    BreakKernel { local_ip: String },
}

/// What happens at one input the script waits for: `changes`, then `input` is typed.
pub struct Step {
    pub changes: Vec<WorldChange>,
    pub input: String,
}

impl Step {
    /// `input` typed after `changes`.
    pub fn new(changes: Vec<WorldChange>, input: &str) -> Self {
        Self {
            changes,
            input: input.to_owned(),
        }
    }
}

/// Runs `script` in the game and in a copy of `snapshot`, taking a step at each input it waits for, and compares the
/// outcomes.
pub async fn compare_stepped(
    client: &TestClient,
    user: &str,
    snapshot: &NetworkSnapshot,
    script: &str,
    steps: &[Step],
) -> Outcome {
    let (output, error) = std::thread::scope(|scope| {
        scope
            .spawn(|| run_stepped_in_snapshot(script, user, snapshot, steps))
            .join()
            .expect("the stepped intrinsics run")
    });
    let expected = Outcome {
        script: script.to_owned(),
        failed: error.is_some(),
        output: error.is_none().then(|| without_ack_counts(&output)),
        error: error.filter(|error| *error != RuntimeErrorKind::GameCrash.to_string()),
    };
    let result = game_stepped(client, script, steps).await;
    let output = result.output.to_string();
    let actual = Outcome {
        script: script.to_owned(),
        failed: result.failed,
        error: result.failed.then(|| game_error(&output)).flatten(),
        output: (!result.failed).then(|| without_ack_counts(&output)),
    };

    assert_eq!(actual, expected);

    actual
}

async fn game_stepped(
    client: &TestClient,
    script: &str,
    steps: &[Step],
) -> greyhack_test_support_client::protocol::test_support::ExecutionResultReply {
    let fail = |error| panic!("{script}: {error}");
    let mut step = client
        .execute_until_input(script, "/project/test.src", "/project", PROGRAM_NAME)
        .await
        .unwrap_or_else(fail);
    let mut steps = steps.iter();

    loop {
        match step {
            ExecutionStep::End(result) => return result,
            ExecutionStep::Input => {
                let next = steps
                    .next()
                    .expect("a step for each input the script waits for");

                for change in &next.changes {
                    change_game(client, change).await;
                }

                step = client.type_input(&next.input).await.unwrap_or_else(fail);
            }
        }
    }
}

async fn change_game(client: &TestClient, change: &WorldChange) {
    match change {
        WorldChange::AdvanceClock(seconds) => {
            client.advance_clock(*seconds).await.unwrap();
        }
        WorldChange::BreakKernel { local_ip } => {
            client.write_router_kernel(local_ip, "{}").await.unwrap();
        }
    }
}

async fn change_world(world: &World, snapshot: &NetworkSnapshot, change: &WorldChange) {
    match change {
        WorldChange::AdvanceClock(seconds) => {
            world.tick(*seconds).await.expect("the clock moves");
        }
        WorldChange::BreakKernel { local_ip } => {
            let router = world
                .modules
                .core
                .networks
                .routers
                .computer_at(&snapshot.public_ip, local_ip)
                .await
                .expect("the device is read")
                .expect("the device");
            let kernel = world
                .modules
                .core
                .files
                .lookup
                .file(router, "/lib/kernel_router.so", false)
                .await
                .expect("the kernel is read")
                .expect("the device's kernel");

            world
                .modules
                .libraries
                .generator
                .set_full_library_of(&kernel, None)
                .await
                .expect("the kernel is replaced");
        }
    }
}

/// Input that asks the driver of [`run_stepped_in_snapshot`] for its line, which it gives after the step's changes.
pub struct SteppedStdin {
    pub requests: mpsc::UnboundedSender<oneshot::Sender<String>>,
}

impl SteppedStdin {
    fn line(&self) -> InputFuture<String> {
        let (reply, line) = oneshot::channel();
        let sent = self.requests.send(reply).is_ok();

        Box::pin(async move {
            Ok(if sent {
                line.await.unwrap_or_default()
            } else {
                String::new()
            })
        })
    }
}

impl Stdin for SteppedStdin {
    fn read_line(&self, _prompt: &str, _echo: greybel_interpreter::Echo) -> InputFuture<String> {
        self.line()
    }

    fn read_key(&self, _prompt: &str) -> InputFuture<Key> {
        let line = self.line();

        Box::pin(async move { Ok(line.await?.chars().next().map_or(Key::Enter, Key::Char)) })
    }

    fn try_read_key(&self, _prompt: &str) -> InputFuture<Option<Key>> {
        Box::pin(std::future::ready(Ok(None)))
    }
}

/// Runs `script` in a copy of `snapshot`, making each step's changes to the world when the script waits for input.
fn run_stepped_in_snapshot(
    script: &str,
    user: &str,
    snapshot: &NetworkSnapshot,
    steps: &[Step],
) -> (String, Option<String>) {
    let world = snapshot_world(user, snapshot);

    duplex_test_support::local::run_local(async move {
        let server = WorldServer::with_database(
            world,
            Database::in_memory().await.expect("an in-memory database"),
            constant_clock(),
        )
        .await
        .expect("the world is written to the database");
        let stdout = Rc::new(MarkedStdout::default());
        let (requests, mut pending) = mpsc::unbounded_channel();
        let options = InterpreterOptions {
            stdout: Rc::clone(&stdout) as Rc<dyn Stdout>,
            stdin: Rc::new(SteppedStdin { requests }),
            ..InterpreterOptions::new(Rc::new(scheduler::TokioScheduler::new()))
        };
        let path = format!("/home/{user}/{PROGRAM_NAME}");
        let driver = async {
            for step in steps {
                let Some(reply) = pending.recv().await else {
                    break;
                };

                server
                    .change_world(async |world| {
                        for change in &step.changes {
                            change_world(world, snapshot, change).await;
                        }
                    })
                    .await;

                let _ = reply.send(step.input.clone());
            }
        };
        let (error, ()) = tokio::join!(
            run_in_at(&server, script, user, true, &path, options),
            driver
        );

        (stdout.terminal_text(), error)
    })
}
