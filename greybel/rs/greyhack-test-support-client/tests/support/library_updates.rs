//! Scripts run in a copy of the game's state after a network's administrator reported a library update.

use greybel_interpreter::RuntimeErrorKind;
use greybel_test_support::future::block_on;
use greybel_test_support::greyhack::run_on_host_at;
use greybel_test_support::interpreter::constant_clock_options;
use greyhack_test_support_client::TestClient;
use greyhack_test_support_client::protocol::test_support::{GuildsReply, NetworkSnapshot};
use greyhack_world_libraries::LibraryReportFailure;

use super::intrinsics::{PROGRAM_NAME, snapshot_world};
use super::mirror::library;
use super::parity::{Outcome, game_error, without_ack_counts};
use super::parity_guilds::mirrored_world;

/// Has the game's administrator of the network at `public_ip` report the newest library file `name`, runs `scripts` in
/// the game, then in a copy of `snapshot`, which was read before the report, in which the simulated administrator
/// reports it too, and asserts both give the same. `newest` is the game's content of the library. Each script is a
/// program of its own, as the game keeps a script's view of a computer it has reached until the script ends.
pub async fn compare_after_report(
    client: &TestClient,
    user: &str,
    snapshot: &NetworkSnapshot,
    name: &str,
    newest: &str,
    guilds: Option<&GuildsReply>,
    scripts: &[String],
) -> Vec<Outcome> {
    let public_ip = snapshot.public_ip.clone();

    let skipped = client
        .report_library_update(&public_ip, name)
        .await
        .expect("the game reports the library")
        .content
        .is_empty();

    let mut actual = Vec::new();
    let mut expected = Vec::new();

    for script in scripts {
        let result = client
            .execute(script, "/project/test.src", "/project", PROGRAM_NAME)
            .await
            .unwrap_or_else(|error| panic!("{script}: {error}"));
        let output = result.output.clone();

        actual.push(Outcome {
            script: script.clone(),
            failed: result.failed,
            error: result.failed.then(|| game_error(&output)).flatten(),
            output: (!result.failed).then(|| without_ack_counts(&output)),
        });

        // On a thread of its own: the run blocks on its own executor, which can't run inside the test's runtime.
        let (output, error) = std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    let world = match guilds {
                        Some(reply) => mirrored_world(user, snapshot, reply),
                        None => snapshot_world(user, snapshot),
                    };

                    block_on(
                        world
                            .modules
                            .libraries
                            .updates
                            .register_library(&library(newest).expect("a library")),
                    )
                    .expect("the game's newest library is registered");

                    let reported = block_on(
                        world
                            .modules
                            .libraries
                            .updates
                            .report_library_update(&public_ip, name),
                    )
                    .expect("the update is reported")
                    .expect("a network");

                    assert_eq!(
                        reported,
                        if skipped {
                            Err(LibraryReportFailure::AdminRestoreOff)
                        } else {
                            Ok(())
                        }
                    );

                    let path = format!("/home/{user}/{PROGRAM_NAME}");
                    let run =
                        run_on_host_at(script, world, user, true, &path, constant_clock_options());

                    (run.terminal, run.error)
                })
                .join()
                .expect("the intrinsics run")
        });

        expected.push(Outcome {
            script: script.clone(),
            failed: error.is_some(),
            output: error.is_none().then(|| without_ack_counts(&output)),
            error: error.filter(|error| *error != RuntimeErrorKind::GameCrash.to_string()),
        });
    }

    assert_eq!(actual, expected);

    actual
}
