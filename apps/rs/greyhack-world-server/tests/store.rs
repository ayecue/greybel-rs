//! Tests the folder in which world servers save the world of each seed.

use std::process::Stdio;
use std::time::Duration;

use duplex_test_support::local::run_local;
use greybel_runtime::SharedWorld;

use crate::support::store::{READ, WRITE, change_formats, databases, files_with_suffix};
use crate::support::{GameServerProcess, run, shared_world, worlds_folder};

#[test]
fn should_keep_the_world_of_a_seed_for_the_next_server() {
    let folder = worlds_folder("keep");
    let arguments = [
        "--seed",
        "kept seed",
        "--worlds",
        folder.to_str().expect("a path"),
    ];
    let first = GameServerProcess::start_with(&arguments, Stdio::inherit());

    run_local(async {
        run(&first.connect().await, WRITE).await;
    });
    first.stop();

    let second = GameServerProcess::start_with(&arguments, Stdio::inherit());

    run_local(async {
        assert_eq!(run(&second.connect().await, READ).await, "1");
    });

    let index = std::fs::read_to_string(folder.join("worlds.json")).expect("the index is written");

    assert!(index.contains("\"kept seed\""), "{index}");
    assert_eq!(databases(&folder).len(), 1);
}

#[test]
fn should_start_the_saved_world_over_on_reset() {
    let folder = worlds_folder("reset");
    let world = shared_world(Duration::from_secs(2), &folder);

    run_local(async {
        let port = world
            .address()
            .rsplit(':')
            .next()
            .and_then(|port| port.parse().ok())
            .expect("a port");

        run(
            &world.connect("test").await.expect("the server starts"),
            WRITE,
        )
        .await;

        let before = databases(&folder);

        SharedWorld::reset(port).await.expect("the world resets");

        assert_eq!(
            run(&world.connect("test").await.expect("the server runs"), READ).await,
            "0"
        );

        let after = databases(&folder);

        assert_eq!(after.len(), 1, "{after:?}");
        assert_ne!(before, after, "the reset replaces the saved world");
    });
}

#[test]
fn should_keep_the_world_in_memory_while_another_server_uses_the_saved_one() {
    let folder = worlds_folder("busy");
    let arguments = ["--worlds", folder.to_str().expect("a path")];
    let first = GameServerProcess::start_with(&arguments, Stdio::inherit());
    let second = GameServerProcess::start_with(&arguments, Stdio::piped());

    run_local(async {
        assert_eq!(run(&second.connect().await, "print 1").await, "1");
    });

    let error = second.stop();

    assert!(
        error.contains("another world server uses the saved world"),
        "{error}"
    );
    drop(first);
}

#[test]
fn should_generate_a_world_again_whose_tables_changed_without_a_version_step() {
    let folder = worlds_folder("changed");
    let arguments = ["--worlds", folder.to_str().expect("a path")];
    let first = GameServerProcess::start_with(&arguments, Stdio::inherit());

    run_local(async {
        run(&first.connect().await, WRITE).await;
    });
    first.stop();
    change_formats(&folder, |format| format["schema"] = "0".into());

    let second = GameServerProcess::start_with(&arguments, Stdio::inherit());

    run_local(async {
        assert_eq!(run(&second.connect().await, READ).await, "0");
    });
    assert_eq!(files_with_suffix(&folder, ".db.outdated").len(), 1);
    assert_eq!(databases(&folder).len(), 1);
}

#[test]
fn should_retain_a_world_of_a_newer_version_and_run_in_memory() {
    let folder = worlds_folder("newer");
    let arguments = ["--worlds", folder.to_str().expect("a path")];
    let first = GameServerProcess::start_with(&arguments, Stdio::inherit());

    run_local(async {
        run(&first.connect().await, WRITE).await;
    });
    first.stop();

    let saved = databases(&folder);

    change_formats(&folder, |format| {
        for version in format["versions"]
            .as_object_mut()
            .expect("versions")
            .values_mut()
        {
            *version = 999.into();
        }
    });

    let second = GameServerProcess::start_with(&arguments, Stdio::piped());

    run_local(async {
        assert_eq!(run(&second.connect().await, READ).await, "0");
    });

    let error = second.stop();

    assert!(error.contains("from a newer Greybel version"), "{error}");
    assert_eq!(databases(&folder), saved);
    assert!(files_with_suffix(&folder, ".outdated").is_empty());
}
