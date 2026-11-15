//! Helpers of the library version parity tests: a version the game released, and the same release in a copy of its state.

use greybel_interpreter::RuntimeErrorKind;
use greybel_test_support::future::block_on;
use greybel_test_support::greyhack::run_on_host_at;
use greybel_test_support::interpreter::constant_clock_options;
use greyhack_behaviour::LibraryKind;
use greyhack_test_support_client::TestClient;
use greyhack_test_support_client::protocol::test_support::{LibraryReleasedReply, NetworkSnapshot};
use greyhack_world::World;

use super::intrinsics::{PROGRAM_NAME, snapshot_world};
use super::mirror::library;
use super::parity::{Outcome, game_error, without_ack_counts};

/// A copy of `snapshot` whose newest libraries are the game's `registered` ones, which the game keeps in
/// `VersionsControl.lastLibs` whether a computer has them or not.
pub fn copy_with_libraries(user: &str, snapshot: &NetworkSnapshot, registered: &[&str]) -> World {
    let world = snapshot_world(user, snapshot);

    for content in registered {
        block_on(
            world
                .modules
                .libraries
                .updates
                .register_library(&library(content).expect("a library of the game")),
        )
        .expect("the game's library is registered");
    }

    world
}

/// Runs `script` in the game, then in the copy `make` builds on a thread of its own (the run blocks on its own executor,
/// which can't run inside the test's runtime), and returns both outcomes.
async fn run_apart(
    client: &TestClient,
    user: &str,
    script: &str,
    make: impl Fn() -> World + Sync,
) -> (Outcome, Outcome) {
    let result = client
        .execute(script, "/project/test.src", "/project", PROGRAM_NAME)
        .await
        .unwrap_or_else(|error| panic!("{script}: {error}"));
    let output = result.output.clone();
    let actual = Outcome {
        script: script.to_owned(),
        failed: result.failed,
        error: result.failed.then(|| game_error(&output)).flatten(),
        output: (!result.failed).then(|| without_ack_counts(&output)),
    };
    let (output, error) = std::thread::scope(|scope| {
        scope
            .spawn(|| {
                let path = format!("/home/{user}/{PROGRAM_NAME}");
                let run =
                    run_on_host_at(script, make(), user, true, &path, constant_clock_options());

                (run.terminal, run.error)
            })
            .join()
            .expect("the intrinsics run")
    });
    let expected = Outcome {
        script: script.to_owned(),
        failed: error.is_some(),
        output: error.is_none().then(|| without_ack_counts(&output)),
        error: error.filter(|error| *error != RuntimeErrorKind::GameCrash.to_string()),
    };

    (actual, expected)
}

/// Runs each of `scripts` in the game and in a copy of `snapshot` whose newest libraries are the game's `registered`
/// ones, asserts both give the same, and returns the game's outcomes. Each script is a program of its own.
pub async fn compare_with_libraries(
    client: &TestClient,
    user: &str,
    snapshot: &NetworkSnapshot,
    registered: &[&str],
    scripts: &[String],
) -> Vec<Outcome> {
    let mut outcomes = Vec::new();

    for script in scripts {
        let (actual, expected) = run_apart(client, user, script, || {
            copy_with_libraries(user, snapshot, registered)
        })
        .await;

        assert_eq!(actual, expected);
        outcomes.push(actual);
    }

    outcomes
}

/// Releases the next version of the library file `name` in the game, which `snapshot` was read before, and in a copy
/// of it that starts with the libraries the game had (`VersionsControl.lastLibs`) and releases it too. Runs each of
/// `scripts` in both, asserts both give the same, and returns the game's release and outcomes.
pub async fn compare_after_release(
    client: &TestClient,
    user: &str,
    snapshot: &NetworkSnapshot,
    name: &str,
    scripts: &[String],
) -> (LibraryReleasedReply, Vec<Outcome>) {
    let released = client
        .release_library_version(name)
        .await
        .expect("the game releases a version");
    let kind = LibraryKind::of_file(name).expect("a library of the game");
    let mut outcomes = Vec::new();

    for script in scripts {
        let (actual, expected) = run_apart(client, user, script, || {
            let world = copy_with_libraries(
                user,
                snapshot,
                &[&released.before, &released.metaxploit_before],
            );

            block_on(
                world
                    .modules
                    .libraries
                    .versions
                    .release_library_version(kind),
            )
            .expect("the version is released")
            .expect("the library exists");

            world
        })
        .await;

        assert_eq!(actual, expected);
        outcomes.push(actual);
    }

    (released, outcomes)
}

/// The official server's library before the release: installed into a folder of its own, checked against itself, with
/// the version a script reads from it.
pub const BEFORE: &str = "apt = include_lib(\"/lib/aptclient.so\"); mx = include_lib(\"/lib/metaxploit.so\"); pc = get_shell.host_computer; print apt.update; print pc.create_folder(home_dir, \"before\"); print apt.install(\"libssh.so\", home_dir + \"/before\"); print mx.load(home_dir + \"/before/libssh.so\").version; print apt.check_upgrade(home_dir + \"/before/libssh.so\")";

/// After the release: the library installed earlier is outdated, and the server gives the new version.
pub const AFTER: &str = "apt = include_lib(\"/lib/aptclient.so\"); mx = include_lib(\"/lib/metaxploit.so\"); pc = get_shell.host_computer; print apt.update; print apt.check_upgrade(home_dir + \"/before/libssh.so\"); print pc.create_folder(home_dir, \"after\"); print apt.install(\"libssh.so\", home_dir + \"/after\"); print mx.load(home_dir + \"/after/libssh.so\").version; print apt.check_upgrade(home_dir + \"/after/libssh.so\")";
