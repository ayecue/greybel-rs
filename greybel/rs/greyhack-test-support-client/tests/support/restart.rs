//! Scripts that run one after another in one world, with the player's computer restarted between them, in the game and
//! in the mirrored world. A restart ends the script that asks for it, so what it did to the world is read by the next.

use std::rc::Rc;

use greybel_interpreter::{InterpreterOptions, RuntimeErrorKind, Stdout};
use greybel_test_support::greyhack::run_in_at;
use greybel_test_support::interpreter::constant_clock;
use greybel_test_support::io::MarkedStdout;
use greyhack_test_support_client::TestClient;
use greyhack_test_support_client::protocol::test_support::NetworkSnapshot;
use greyhack_world::Database;
use greyhack_world_operations::WorldServer;

use super::intrinsics::{PROGRAM_NAME, snapshot_world};
use super::parity::{Outcome, game_error, without_ack_counts};

/// What happens next: a script runs, or the player's computer restarts as `ComputerIntrinsics.reboot` does from a
/// terminal connected to it (the game's client starting it again).
#[derive(Clone, Copy)]
pub enum Phase<'a> {
    Script(&'a str),
    Restart { safe_mode: bool },
}

/// Runs `phases` in order in the game and in one copy of `snapshot`, asserts that every script's outcome is the same
/// and returns the game's outcomes, one for each script.
pub async fn compare_with_restarts(
    client: &TestClient,
    user: &str,
    snapshot: &NetworkSnapshot,
    phases: &[Phase<'_>],
) -> Vec<Outcome> {
    let expected = std::thread::scope(|scope| {
        scope
            .spawn(|| run_in_snapshot(user, snapshot, phases))
            .join()
            .expect("the intrinsics run")
    });
    let expected: Vec<Outcome> = phases
        .iter()
        .filter_map(|phase| match phase {
            Phase::Script(script) => Some(*script),
            Phase::Restart { .. } => None,
        })
        .zip(expected)
        .map(|(script, (output, error))| Outcome {
            script: script.to_owned(),
            failed: error.is_some(),
            output: error.is_none().then(|| without_ack_counts(&output)),
            error: error.filter(|error| *error != RuntimeErrorKind::GameCrash.to_string()),
        })
        .collect();
    let mut actual = Vec::new();

    for phase in phases {
        match phase {
            Phase::Script(script) => {
                let result = client
                    .execute(script, "/project/test.src", "/project", PROGRAM_NAME)
                    .await
                    .unwrap_or_else(|error| panic!("{script}: {error}"));
                let output = result.output.to_string();

                actual.push(Outcome {
                    script: (*script).to_owned(),
                    failed: result.failed,
                    error: result.failed.then(|| game_error(&output)).flatten(),
                    output: (!result.failed).then(|| without_ack_counts(&output)),
                });
            }
            Phase::Restart { safe_mode } => {
                client
                    .reboot_player(*safe_mode)
                    .await
                    .unwrap_or_else(|error| panic!("the player's computer restarts: {error}"));
            }
        }
    }

    assert_eq!(actual, expected);

    actual
}

/// The terminal text and runtime error of each script of `phases`, run in one copy of `snapshot`.
fn run_in_snapshot(
    user: &str,
    snapshot: &NetworkSnapshot,
    phases: &[Phase<'_>],
) -> Vec<(String, Option<String>)> {
    let world = snapshot_world(user, snapshot);

    duplex_test_support::local::run_local(async move {
        let server = WorldServer::with_database(
            world,
            Database::in_memory().await.expect("an in-memory database"),
            constant_clock(),
        )
        .await
        .expect("the world is written to the database");
        let path = format!("/home/{user}/{PROGRAM_NAME}");
        let mut outcomes = Vec::new();

        for phase in phases {
            match phase {
                Phase::Script(script) => {
                    let stdout = Rc::new(MarkedStdout::default());
                    let options = InterpreterOptions {
                        stdout: Rc::clone(&stdout) as Rc<dyn Stdout>,
                        ..InterpreterOptions::new(Rc::new(scheduler::TokioScheduler::new()))
                    };
                    let error = run_in_at(&server, script, user, true, &path, options).await;

                    outcomes.push((stdout.terminal_text(), error));
                }
                Phase::Restart { safe_mode } => {
                    server
                        .change_world(async |world| {
                            assert!(
                                world
                                    .reboot_player(*safe_mode)
                                    .await
                                    .expect("the player's computer restarts")
                            );
                        })
                        .await;
                }
            }
        }

        outcomes
    })
}
