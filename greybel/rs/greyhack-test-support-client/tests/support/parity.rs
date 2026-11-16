//! Compares outcomes of the same scripts in live Grey Hack and the intrinsics.

use std::future::Future;
use std::sync::Arc;

use greybel_interpreter::RuntimeErrorKind;
use greyhack_test_support_client::protocol::test_support::NetworkSnapshot;
use greyhack_test_support_client::{ExecutionStep, TestClient};

use super::intrinsics::{
    PROGRAM_NAME, run_interactive_action_in_snapshot, run_interactive_in_snapshot,
    run_many_in_snapshot, run_many_in_snapshot_real_time, run_many_in_snapshot_with_floods,
    run_many_in_snapshot_with_wait_clock, run_many_in_snapshots, run_many_on_new_player,
    run_pair_in_snapshot,
};

/// Script outcome with a failure flag, output present only on success and the error message on failure.
#[derive(Debug, PartialEq, Eq)]
pub struct Outcome {
    pub script: String,
    pub failed: bool,
    pub output: Option<String>,
    pub error: Option<String>,
}

/// The message of the runtime error the game printed in `output`, on a line of its own as
/// `<color=red>Runtime Error: <message> [<file> line <n>]</color>`.
pub fn game_error(output: &str) -> Option<String> {
    let message = output.lines().find_map(|line| {
        line.strip_prefix("<color=red>Runtime Error: ")?
            .strip_suffix("</color>")
    })?;
    let message = match message.rfind(" [") {
        Some(at) if message.ends_with(']') => &message[..at],
        _ => message,
    };

    Some(message.trim_end().to_owned())
}

/// Executes each script in the game through `client` and returns outcomes in input order.
/// `output` with the counts of `aireplay`'s progress line taken out: both the game and the world draw them at random
/// (`Terminal.CollectData`), so only the line itself compares.
pub fn without_ack_counts(output: &str) -> String {
    output
        .split('\n')
        .map(|line| {
            let counts = line
                .strip_prefix("Read ")
                .and_then(|rest| rest.strip_suffix(" ACKs)"))
                .and_then(|rest| rest.split_once(" packets (got "));

            match counts {
                Some((packets, acks))
                    if packets.bytes().all(|byte| byte.is_ascii_digit())
                        && acks.bytes().all(|byte| byte.is_ascii_digit()) =>
                {
                    "Read <packets> packets (got <acks> ACKs)"
                }
                _ => line,
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub async fn game_outcomes(client: &TestClient, scripts: &[&str]) -> Vec<Outcome> {
    let mut outcomes = Vec::with_capacity(scripts.len());

    for script in scripts {
        let result = client
            .execute(script, "/project/test.src", "/project", PROGRAM_NAME)
            .await
            .unwrap_or_else(|error| panic!("{script}: {error}"));

        let output = result.output.clone();

        outcomes.push(Outcome {
            script: (*script).to_owned(),
            failed: result.failed,
            error: result.failed.then(|| game_error(&output)).flatten(),
            output: (!result.failed).then(|| without_ack_counts(&output)),
        });
    }

    outcomes
}

/// Executes an interactive script on both sides, with the fixture typing `inputs` after each `ready` print.
pub async fn compare_interactive(
    client: &TestClient,
    user: &str,
    snapshot: &NetworkSnapshot,
    script: &str,
    inputs: &[String],
) {
    let (output, error) = std::thread::scope(|scope| {
        scope
            .spawn(|| run_interactive_in_snapshot(script, user, snapshot, inputs))
            .join()
            .expect("the interactive intrinsics run")
    });
    let expected = Outcome {
        script: script.to_owned(),
        failed: error.is_some(),
        output: error.is_none().then(|| without_ack_counts(&output)),
        error: error.filter(|error| *error != RuntimeErrorKind::GameCrash.to_string()),
    };
    let result = client
        .execute_interactive(
            script,
            "/project/test.src",
            "/project",
            PROGRAM_NAME,
            inputs,
        )
        .await
        .unwrap_or_else(|error| panic!("{script}: {error}"));
    let output = result.output.clone();
    let actual = Outcome {
        script: script.to_owned(),
        failed: result.failed,
        error: result.failed.then(|| game_error(&output)).flatten(),
        output: (!result.failed).then(|| without_ack_counts(&output)),
    };

    assert_eq!(actual, expected);
}

/// Runs an interactive script with an action before each input on both sides.
pub async fn compare_interactive_action<F>(
    client: &TestClient,
    user: &str,
    snapshot: &NetworkSnapshot,
    script: &str,
    inputs: &[String],
    mirror_action: Arc<dyn Fn(&greyhack_world::World) + Send + Sync>,
    mut game_action: impl FnMut() -> F,
) where
    F: Future<Output = ()> + Send,
{
    let (output, error) = std::thread::scope(|scope| {
        scope
            .spawn(|| {
                run_interactive_action_in_snapshot(script, user, snapshot, inputs, mirror_action)
            })
            .join()
            .expect("the interactive action run")
    });
    let expected = Outcome {
        script: script.to_owned(),
        failed: error.is_some(),
        output: error.is_none().then(|| without_ack_counts(&output)),
        error: error.filter(|error| *error != RuntimeErrorKind::GameCrash.to_string()),
    };
    let mut step = client
        .execute_until_input(script, "/project/test.src", "/project", PROGRAM_NAME)
        .await
        .unwrap_or_else(|error| panic!("{script}: {error}"));
    let mut inputs = inputs.iter();
    let result = loop {
        match step {
            ExecutionStep::Input => {
                game_action().await;
                step = client
                    .type_input(inputs.next().map_or("", String::as_str))
                    .await
                    .unwrap_or_else(|error| panic!("{script}: {error}"));
            }
            ExecutionStep::End(result) => break result,
        }
    };
    let output = result.output.clone();
    let actual = Outcome {
        script: script.to_owned(),
        failed: result.failed,
        error: result.failed.then(|| game_error(&output)).flatten(),
        output: (!result.failed).then(|| without_ack_counts(&output)),
    };

    assert_eq!(actual, expected);
}

/// Executes two scripts concurrently in independent game contexts.
pub async fn game_pair_outcomes(client: &TestClient, scripts: [&str; 2]) -> [Outcome; 2] {
    let result = client
        .execute_pair(
            scripts[0],
            scripts[1],
            "/project/test.src",
            "/project",
            PROGRAM_NAME,
        )
        .await
        .unwrap_or_else(|error| panic!("{scripts:?}: {error}"));
    let results = [
        (result.first_output, result.first_failed),
        (result.second_output, result.second_failed),
    ];

    results
        .into_iter()
        .enumerate()
        .map(|(index, (output, failed))| {
            let output = output.clone();

            Outcome {
                script: scripts[index].to_owned(),
                failed,
                error: failed.then(|| game_error(&output)).flatten(),
                output: (!failed).then(|| without_ack_counts(&output)),
            }
        })
        .collect::<Vec<_>>()
        .try_into()
        .expect("two outcomes")
}

/// Runs each script through the intrinsics as admin `user` and returns outcomes in input order.
pub fn intrinsics_outcomes(scripts: &[&str], user: &str) -> Vec<Outcome> {
    outcomes(scripts, |scripts| run_many_on_new_player(scripts, user))
}

/// Runs each script as [`intrinsics_outcomes`] does, in the simulated copy of `snapshot`: the player's network with
/// its NPCs, the game's date and the global economy. Never only the network's layout: NPC logins decide active users,
/// phones and administrators. All scripts share one world, in input order.
pub fn snapshot_outcomes(scripts: &[&str], user: &str, snapshot: &NetworkSnapshot) -> Vec<Outcome> {
    outcomes(scripts, |scripts| {
        run_many_in_snapshot(scripts, user, snapshot)
    })
}

/// The outcome of every script run, in input order.
fn outcomes(
    scripts: &[&str],
    run: impl Fn(&[&str]) -> Vec<(String, Option<String>)> + Sync,
) -> Vec<Outcome> {
    // On a thread of its own: the run blocks on its own executor, which can't run inside the test's runtime.
    let results = std::thread::scope(|scope| {
        scope
            .spawn(|| run(scripts))
            .join()
            .expect("the intrinsics run")
    });

    outcomes_from_runs(scripts, results)
}

/// Converts each script's printed output and error into an [`Outcome`].
pub fn outcomes_from_runs(
    scripts: &[&str],
    results: Vec<(String, Option<String>)>,
) -> Vec<Outcome> {
    scripts
        .iter()
        .zip(results)
        .map(|(script, (output, error))| Outcome {
            script: (*script).to_owned(),
            failed: error.is_some(),
            output: error.is_none().then(|| without_ack_counts(&output)),
            error: error.filter(|error| *error != RuntimeErrorKind::GameCrash.to_string()),
        })
        .collect()
}

/// [`snapshot_outcomes`] with real waits and world ticks, for stopwatch-based behavior.
fn real_time_snapshot_outcomes(
    scripts: &[&str],
    user: &str,
    snapshot: &NetworkSnapshot,
) -> Vec<Outcome> {
    outcomes(scripts, |scripts| {
        run_many_in_snapshot_real_time(scripts, user, snapshot)
    })
}

/// Runs `scripts` in the game and in a simulated network laid out like `snapshot`, asserts both give the same, and
/// returns the game's outcomes for fixture-specific assertions.
pub async fn compare_in(
    client: &TestClient,
    user: &str,
    snapshot: &NetworkSnapshot,
    scripts: &[String],
) -> Vec<Outcome> {
    let scripts: Vec<&str> = scripts.iter().map(String::as_str).collect();
    let expected = snapshot_outcomes(&scripts, user, snapshot);

    let actual = game_outcomes(client, &scripts).await;

    assert_eq!(actual, expected);

    actual
}

/// [`compare_in`] with the fixture state included in a failed assertion.
pub async fn compare_labeled(
    client: &TestClient,
    user: &str,
    snapshot: &NetworkSnapshot,
    label: &str,
    scripts: &[String],
) -> Vec<Outcome> {
    let scripts: Vec<&str> = scripts.iter().map(String::as_str).collect();
    let expected = snapshot_outcomes(&scripts, user, snapshot);
    let actual = game_outcomes(client, &scripts).await;

    assert_eq!(actual, expected, "{label}");

    actual
}

/// [`compare_labeled`] with additional networks mirrored without joining them.
pub async fn compare_labeled_with_networks(
    client: &TestClient,
    user: &str,
    base: &NetworkSnapshot,
    additional: &[NetworkSnapshot],
    label: &str,
    script: &str,
) {
    let expected = outcomes(&[script], |scripts| {
        run_many_in_snapshots(scripts, user, base, additional)
    });
    let actual = game_outcomes(client, &[script]).await;

    assert_eq!(actual, expected, "{label}");
}

/// [`compare_in`] after priming the same flood counts as the game fixture.
pub async fn compare_in_flooded(
    client: &TestClient,
    user: &str,
    snapshot: &NetworkSnapshot,
    scripts: &[String],
    floods: &[(String, String, u32)],
) -> Vec<Outcome> {
    compare_in_flooded_labeled(client, user, snapshot, "flooded", scripts, floods).await
}

/// [`compare_in_flooded`] with the fixture state included in a failed assertion.
pub async fn compare_in_flooded_labeled(
    client: &TestClient,
    user: &str,
    snapshot: &NetworkSnapshot,
    label: &str,
    scripts: &[String],
    floods: &[(String, String, u32)],
) -> Vec<Outcome> {
    let scripts: Vec<&str> = scripts.iter().map(String::as_str).collect();
    let expected = outcomes(&scripts, |scripts| {
        run_many_in_snapshot_with_floods(scripts, user, snapshot, floods)
    });

    let actual = game_outcomes(client, &scripts).await;

    assert_eq!(actual, expected, "{label}");

    actual
}

/// Runs two scripts concurrently in the game and in one mirrored world, and asserts both pairs match.
pub async fn compare_pair_in(
    client: &TestClient,
    user: &str,
    snapshot: &NetworkSnapshot,
    scripts: [&str; 2],
) {
    compare_pair_labeled(client, user, snapshot, "concurrent pair", scripts).await;
}

/// [`compare_pair_in`] with the fixture state included in a failed assertion.
pub async fn compare_pair_labeled(
    client: &TestClient,
    user: &str,
    snapshot: &NetworkSnapshot,
    label: &str,
    scripts: [&str; 2],
) {
    let expected = std::thread::scope(|scope| {
        scope
            .spawn(|| run_pair_in_snapshot(scripts[0], scripts[1], user, snapshot))
            .join()
            .expect("the concurrent intrinsics run")
    });
    let expected = expected
        .into_iter()
        .enumerate()
        .map(|(index, (output, error))| Outcome {
            script: scripts[index].to_owned(),
            failed: error.is_some(),
            output: error.is_none().then(|| without_ack_counts(&output)),
            error: error.filter(|error| *error != RuntimeErrorKind::GameCrash.to_string()),
        });
    let actual = game_pair_outcomes(client, scripts).await;

    assert_eq!(
        actual.into_iter().collect::<Vec<_>>(),
        expected.collect::<Vec<_>>(),
        "{label}"
    );
}

/// [`compare_in`] where `wait` advances the program's clock but not the world's, for the call throttles of intrinsics.
pub async fn compare_in_on_waiting_clock(
    client: &TestClient,
    user: &str,
    snapshot: &NetworkSnapshot,
    scripts: &[String],
) -> Vec<Outcome> {
    let scripts: Vec<&str> = scripts.iter().map(String::as_str).collect();
    let expected = outcomes(&scripts, |scripts| {
        run_many_in_snapshot_with_wait_clock(scripts, user, snapshot)
    });
    let actual = game_outcomes(client, &scripts).await;

    assert_eq!(actual, expected);

    actual
}

/// [`compare_in`] with real waits and world ticks on both sides.
pub async fn compare_in_real_time(
    client: &TestClient,
    user: &str,
    snapshot: &NetworkSnapshot,
    scripts: &[String],
) -> Vec<Outcome> {
    let scripts: Vec<&str> = scripts.iter().map(String::as_str).collect();
    let expected = real_time_snapshot_outcomes(&scripts, user, snapshot);

    let actual = game_outcomes(client, &scripts).await;

    assert_eq!(actual, expected);

    actual
}

/// What an attack prints before the user it gives privileges of.
const PRIVILEGES: &str = "Privileges obtained from user: ";

/// What an attack prints before the user whose credentials a computer reward uses.
const CREDENTIALS: &str = "Computer obtained with credentials from user: ";

/// What a password reward prints before the user whose password it can't change.
const PASSWORD: &str = "error: can't change password for user ";

/// [`compare_in`] for attacks on a weakness of the game's `normal_user`, whose user the game draws at random
/// (`ScriptsUtil.GetShellUser`): the user the privileges or credentials are of is left out on both sides.
pub async fn compare_in_any_user(
    client: &TestClient,
    user: &str,
    snapshot: &NetworkSnapshot,
    scripts: &[String],
) -> Vec<Outcome> {
    let scripts: Vec<&str> = scripts.iter().map(String::as_str).collect();
    let expected = without_users(snapshot_outcomes(&scripts, user, snapshot));

    let actual = without_users(game_outcomes(client, &scripts).await);

    assert_eq!(actual, expected);

    actual
}

/// `outcomes` without the user named after [`PRIVILEGES`], [`CREDENTIALS`] or [`PASSWORD`], which the game draws at
/// random for a weakness of `normal_user` (`ScriptsUtil.GetShellUser`).
pub fn without_users(outcomes: Vec<Outcome>) -> Vec<Outcome> {
    outcomes
        .into_iter()
        .map(|mut outcome| {
            outcome.output = outcome.output.map(|output| {
                output
                    .split('\n')
                    .map(without_user)
                    .collect::<Vec<_>>()
                    .join("\n")
            });

            outcome
        })
        .collect()
}

/// `line` without the user it names after one of the prefixes of [`without_users`].
fn without_user(line: &str) -> String {
    if let Some(prefix) = [PRIVILEGES, CREDENTIALS]
        .into_iter()
        .find(|prefix| line.starts_with(prefix))
    {
        return prefix.to_owned();
    }

    match line
        .strip_prefix(PASSWORD)
        .and_then(|rest| rest.split_once(". "))
    {
        Some((_, after)) => format!("{PASSWORD}. {after}"),
        None => line.to_owned(),
    }
}

/// Fails when a script failed, so a comparison of two equal failures can't pass for the wrong reason.
pub fn assert_succeeded(outcomes: &[Outcome]) {
    assert!(
        outcomes.iter().all(|outcome| !outcome.failed),
        "{outcomes:?}"
    );
}

/// Fails when the first or last line of `output` is empty: `RouterIntrinsics.port_info` answers null, printed as an
/// empty line, when `ping_port` found no port object.
pub fn assert_port_info_lines(output: Option<&str>) {
    let output = output.expect("a script that succeeded has output");
    let lines: Vec<&str> = output.trim_end_matches('\n').split('\n').collect();

    assert!(
        lines.first().is_some_and(|line| !line.is_empty())
            && lines.last().is_some_and(|line| !line.is_empty()),
        "port_info printed nothing: {output:?}"
    );
}
