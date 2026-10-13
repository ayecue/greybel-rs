//! Snapshots script output and intrinsic errors; update with `INSTA_UPDATE=always cargo test`.

use duplex_test_support::local::run_local;
use greybel_test_support::future::block_on;
use greybel_test_support::greyhack;
use greybel_test_support::interpreter::{constant_clock, snapshot_options};
use greybel_test_support::world::persistent_fingerprint;
use greyhack_world::{Database, PlayerOptions, World};
use greyhack_world_operations::WorldServer;

use crate::support::scripts::{
    CHANGES, CHECKS, ERRORS, GAME_ERRORS, printed, reported_errors, run_generated, run_on_host,
    run_opened,
};
use crate::support::world_options;

#[test]
fn should_print_what_every_script_fixture_prints() {
    insta::glob!("fixtures/scripts/*.src", |path| {
        let code = std::fs::read_to_string(path).expect("a fixture");
        let output = printed(&code, false);
        insta::assert_snapshot!(output);
    });
}

/// Each program's error, raised or returned as its value: Grey Hack's intrinsics return most of their errors.
#[test]
fn should_report_every_intrinsic_error() {
    let snapshot = reported_errors(ERRORS, false);

    insta::assert_snapshot!(snapshot);
}

#[test]
fn should_print_what_every_game_script_fixture_prints() {
    insta::glob!("fixtures/game/*.src", |path| {
        let code = std::fs::read_to_string(path).expect("a fixture");
        let output = printed(&code, true);
        insta::assert_snapshot!(output);
    });
}

#[test]
fn should_report_every_game_intrinsic_error() {
    let snapshot = reported_errors(GAME_ERRORS, true);

    insta::assert_snapshot!(snapshot);
}

/// A run saves the world to the database at its end; opening a fresh
/// interpreter from that database continues the same world.
#[test]
fn should_print_nothing_while_changing_the_world() {
    block_on(async {
        let options = world_options(9, 2);
        let database = Database::in_memory().await.expect("database");
        let output = run_generated(&options, &database, CHANGES);

        assert_eq!(output, String::new());
    });
}

#[test]
fn should_continue_the_saved_world_in_a_fresh_interpreter() {
    block_on(async {
        let options = world_options(9, 2);
        let database = Database::in_memory().await.expect("database");

        run_generated(&options, &database, CHANGES);

        let output = run_opened(&database, CHECKS);

        assert_eq!(output, "1\n1\n4");
    });
}

#[test]
fn should_not_see_the_saved_changes_in_a_fresh_world() {
    block_on(async {
        let options = world_options(9, 2);
        let database = Database::in_memory().await.expect("database");

        run_generated(&options, &database, CHANGES);
        run_opened(&database, CHECKS);

        let output = run_generated(
            &options,
            &database,
            r#"
                print get_shell.host_computer.File(home_dir + "/kept.txt") != null
                print include_lib("admin.so").networks.indexOf("5.5.5.5") == null
                print include_lib("admin.so").networks.len
            "#,
        );

        assert_eq!(output, "0\n1\n3");
    });
}

/// `admin.snapshot` writes the live world to the database, so `World::load`
/// reads back exactly what a direct save would.
#[test]
fn should_print_success_of_an_admin_snapshot() {
    block_on(async {
        let options = world_options(4, 2);
        let database = Database::in_memory().await.expect("database");
        let output = run_generated(
            &options,
            &database,
            "print include_lib(\"admin.so\").snapshot",
        );

        assert_eq!(output, "1");
    });
}

#[test]
fn should_load_what_an_admin_snapshot_wrote() {
    run_local(async {
        let database = Database::in_memory().await.expect("database");
        let server = WorldServer::with_database(
            World::generate(&world_options(4, 2))
                .await
                .expect("a generated world"),
            database,
            constant_clock(),
        )
        .await
        .expect("the world is written to the database");
        let (_, options) = snapshot_options();
        let user = PlayerOptions::default().user;
        let code = "print include_lib(\"admin.so\").snapshot";

        greyhack::run_in(&server, code, &user, true, options).await;

        let reloaded = World::load(server.database().expect("a database"))
            .await
            .expect("load");

        let live = server.world().await;

        assert_eq!(
            persistent_fingerprint(&reloaded).await,
            persistent_fingerprint(&live).await
        );
    });
}

#[test]
fn should_turn_admin_so_off() {
    let world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let (output, _, _) = run_on_host("print include_lib(\"admin.so\")", world, false);

    assert_eq!(output, "null");
}
