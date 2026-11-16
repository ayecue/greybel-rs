//! Debug libraries: their challenges, patches and lockouts.

use greybel_test_support::game::Game;
use greyhack_behaviour::{SECONDS_PER_DAY, add_month, add_months, date_parts, parse_date};

use crate::support::game::{
    join_fixture, join_network, setup_player, ssh_network_of_kind, test_client,
};
use crate::support::intrinsics::snapshot_world;
use crate::support::parity::{compare_in, compare_labeled, compare_labeled_with_networks};
use crate::support::remote::{fixture_account, script_string};
use crate::support::steps::{Step, WorldChange, compare_stepped};

/// The debug challenge generated for the player's metaxploit library: inspecting and testing it, acquiring its partial
/// objects, and applying its patch. The same script performs every step on both sides because each changes the library.
#[tokio::test]
async fn should_match_intrinsics_for_debug_libraries() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;

    setup_player(&client).await;
    client.setup_debug().await.unwrap();
    client.freeze_clock().await.unwrap();

    let snapshot = client.network_state().await.unwrap();
    let debug = snapshot.debug.as_ref().expect("the debug fixture");
    let account = format!(
        "{}, {}",
        script_string(&debug.user),
        script_string(&debug.password)
    );
    let lines = format!("{:?}", debug.error_lines);
    let zone = script_string(&debug.memory_zone);
    // `CodeComparer` accepts this normalized form of the generated correct code too.
    let normalized = debug
        .correct_code
        .chars()
        .filter(|character| !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect::<String>();
    let normalized = script_string(&normalized);
    let now = parse_date(&snapshot.current_date);
    let (_, _, day) = date_parts(now);
    let month = now - (u64::from(day - 1) * SECONDS_PER_DAY + now % SECONDS_PER_DAY);
    let next_month = add_month(month);
    let auto_patch = add_months(month, 3);
    let past_auto_patch = auto_patch + SECONDS_PER_DAY;
    let future_world = snapshot_world("root", &snapshot);

    future_world.tick(past_auto_patch - now).await.unwrap();

    let future = future_world
        .modules
        .libraries
        .debug
        .debug_library(future_world.handle.player(), &debug.path)
        .await
        .unwrap()
        .unwrap();
    let future_zone = script_string(&future.zero_day().address);
    let future_challenge = future.challenge();
    let future_lines = format!("{:?}", future_challenge.error_lines);
    let future_normalized = future_challenge
        .correct_code
        .chars()
        .filter(|character| !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect::<String>();
    let future_normalized = script_string(&future_normalized);
    let script = format!(
        r#"mx = include_lib("/lib/metaxploit.so")
lib = mx.load("/lib/metaxploit.so")
print [lib.is_patched, lib.is_patched(1)]
debug = lib.debug_tools({account})
print typeof(debug)
print debug.scan_debuglib
print debug.unit_testing({lines} + [{lines}[0]])
print debug.payload("0xBAD")
print debug.payload({zone}, home_dir + "/missing")
objects = debug.payload({zone})
print objects.len
for object in objects
  print typeof(object)
end for
pc = get_shell.host_computer
print pc.File("/lib/metaxploit.so").copy(home_dir, "debugcopy.so")
outside = mx.load(home_dir + "/debugcopy.so").debug_tools({account})
print outside.payload({zone})
pc.touch(home_dir, "patch.src")
patch = pc.File(home_dir + "/patch.src")
print debug.apply_patch(home_dir + "/missing.src")
print debug.apply_patch("/bin/ls")
patch.set_content("wrong")
print debug.apply_patch(patch.path)
patch.set_content({normalized})
print debug.apply_patch(patch.path)
print debug.apply_patch(patch.path)
print [lib.is_patched, lib.is_patched(1)]
print debug.scan_debuglib
print debug.unit_testing({lines})
print debug.payload({zone})
user_input("")
print "next month"
print [lib.is_patched, lib.is_patched(1)]
user_input("")
print "past automatic patch"
print [lib.is_patched, lib.is_patched(1)]
debug = lib.debug_tools({account})
print debug.scan_debuglib
print debug.unit_testing({future_lines} + [{future_lines}[0]])
print debug.payload({future_zone}).len
print [lib.is_patched, lib.is_patched(1)]
patch.set_content({future_normalized})
print debug.apply_patch(patch.path)
print [lib.is_patched, lib.is_patched(1)]"#
    );

    compare_stepped(
        &client,
        "root",
        &snapshot,
        &script,
        &[
            Step::new(vec![WorldChange::AdvanceClock(next_month - now)], ""),
            Step::new(
                vec![WorldChange::AdvanceClock(past_auto_patch - next_month)],
                "",
            ),
        ],
    )
    .await;
    client.shutdown();
}

/// Failed debug logins lock the account on the fourth attempt, and a wrong unit test blocks later tests until the
/// game's monthly reset (`ZeroDayPlayerInfo`).
#[tokio::test]
async fn should_match_intrinsics_for_debug_library_lockouts() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;

    setup_player(&client).await;
    client.setup_debug().await.unwrap();

    let snapshot = client.network_state().await.unwrap();
    let debug = snapshot.debug.as_ref().expect("the debug fixture");
    let account = format!(
        "{}, {}",
        script_string(&debug.user),
        script_string(&debug.password)
    );
    let debug_user = script_string(&debug.user);
    let lines = format!("{:?}", debug.error_lines);
    let script = format!(
        r#"mx = include_lib("/lib/metaxploit.so")
lib = mx.load("/lib/metaxploit.so")
debug = lib.debug_tools({account})
print debug.unit_testing([999999])
debug = lib.debug_tools({account})
print debug.unit_testing({lines})
for i in range(1, 4)
  print lib.debug_tools({debug_user}, "wrong")
end for
print lib.debug_tools({account})"#
    );

    compare_in(&client, "root", &snapshot, &[script]).await;
    client.shutdown();
}

/// Wrong debug argument types and detached debug-library helpers.
#[tokio::test]
async fn should_match_intrinsics_for_debug_argument_errors() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;

    setup_player(&client).await;
    client.setup_debug().await.unwrap();

    let snapshot = client.network_state().await.unwrap();
    let debug = snapshot.debug.as_ref().unwrap();
    let account = format!(
        "{}, {}",
        script_string(&debug.user),
        script_string(&debug.password)
    );
    let script = format!(
        "mx = include_lib(\"/lib/metaxploit.so\"); lib = mx.load(\"/lib/metaxploit.so\"); debug = lib.debug_tools({account}); print [lib.debug_tools(1), lib.is_patched(\"x\"), debug.unit_testing(1), debug.payload(1), debug.apply_patch(1)]; print [apply_patch({{}}, \"x\"), payload({{}}, \"0x0\"), unit_testing({{}}, []), scan_debuglib({{}})]"
    );

    compare_labeled(&client, "root", &snapshot, "debug-arguments", &[script]).await;
    client.shutdown();
}

/// Existing MetaLib and DebugLibrary objects see a replacement that is not a library.
#[tokio::test]
async fn should_match_intrinsics_for_non_library_debug_objects() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;

    setup_player(&client).await;
    client.setup_debug().await.unwrap();

    let network = ssh_network_of_kind(&client, "RentServer").await;
    let account = fixture_account(&network, "ssh", "ssh", false);
    let snapshot = join_fixture(&client, &network).await;

    assert!(snapshot.admin.is_none(), "debug-non-library");

    let debug = snapshot.debug.as_ref().unwrap();
    let credentials = format!(
        "{}, {}",
        script_string(&debug.user),
        script_string(&debug.password)
    );
    let script = format!(
        "remote = get_shell.connect_service(\"{}\", {}, \"root\", \"{}\"); pc = remote.host_computer; mx = include_lib(\"/lib/metaxploit.so\"); session = mx.net_use(\"{}\", {}); lib = session.dump_lib; debug = lib.debug_tools({credentials}); pc.touch(\"/root\", \"notlib.txt\"); pc.File(\"/root/notlib.txt\").move(\"/lib\", \"libssh.so\"); print lib.debug_tools({credentials}); print lib.is_patched; print debug.scan_debuglib; print debug.unit_testing([]); print debug.payload(\"0x0\"); print debug.apply_patch(\"/root/notlib.txt\")",
        account.address, account.port, account.root_password, account.address, account.port
    );

    compare_labeled(&client, "root", &snapshot, "debug-non-library", &[script]).await;
    client.shutdown();
}

/// The first remote debug login binds its network; another remote network and a limited local user are rejected.
#[tokio::test]
async fn should_match_intrinsics_for_remote_debug_network_binding() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;

    setup_player(&client).await;
    client.setup_debug().await.unwrap();
    client.configure_computer("", "permissions").await.unwrap();

    let home = client.network_state().await.unwrap();
    let first = ssh_network_of_kind(&client, "RentServer").await;
    let first_account = fixture_account(&first, "ssh", "ssh", true);
    let first_snapshot = join_fixture(&client, &first).await;
    let second = ssh_network_of_kind(&client, "RentServer").await;
    let second_account = fixture_account(&second, "ssh", "ssh", true);
    let second_snapshot = join_fixture(&client, &second).await;

    join_network(&client, &home.public_ip).await;

    let base = client.network_state().await.unwrap();
    let debug = base.debug.as_ref().unwrap();
    let credentials = format!(
        "{}, {}",
        script_string(&debug.user),
        script_string(&debug.password)
    );
    let script = format!(
        "mx = include_lib(\"/lib/metaxploit.so\"); first = mx.net_use(\"{}\", {}).dump_lib; print first.debug_tools({credentials}); second = mx.net_use(\"{}\", {}).dump_lib; print second.debug_tools({credentials}); get_shell(\"limited\"); local = include_lib(\"/lib/metaxploit.so\").load(\"/lib/libssh.so\"); print local.debug_tools({credentials})",
        first_account.address, first_account.port, second_account.address, second_account.port
    );

    compare_labeled_with_networks(
        &client,
        "root",
        &base,
        &[first_snapshot, second_snapshot],
        "debug-remote-binding",
        &script,
    )
    .await;
    client.shutdown();
}
