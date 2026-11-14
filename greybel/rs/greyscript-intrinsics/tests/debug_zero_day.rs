use greybel_test_support::future::block_on;
use greyhack_behaviour::Requirement;
use greyhack_world::{DebugAccount, World};

use crate::support::debug::debug_world;
use crate::support::save_copy;
use crate::support::scripts::run_on_host;

#[test]
fn should_run_unit_testing_and_lock_out_a_failed_attempt() {
    let world = debug_world();

    block_on(
        world
            .modules
            .libraries
            .debug
            .save_debug_account(&DebugAccount::new("test", "test")),
    )
    .unwrap();

    let code = r#"
meta = include_lib("/lib/metaxploit.so")
lib = meta.load("/lib/metaxploit.so")
debug = lib.debug_tools("dbg", "secret")
print debug.unit_testing([999999])
print debug.unit_testing([1])
"#;

    let (output, error, restored) = run_on_host(code, world, true);

    assert_eq!(error, None);
    assert!(
        output.contains("Suspicious activity on the API. Access blocked."),
        "{output}"
    );
    assert!(
        output.contains("Error: Previous unit test failed. Access denied."),
        "{output}"
    );
    assert!(
        block_on(restored.modules.libraries.debug.debug_account("test"))
            .unwrap()
            .unwrap()
            .unit_test_failed
    );
}

#[test]
fn should_report_a_zero_day_and_create_partial_payload_objects() {
    let world = debug_world();

    block_on(
        world
            .modules
            .libraries
            .debug
            .save_debug_account(&DebugAccount::new("test", "test")),
    )
    .unwrap();

    let address = block_on(
        world
            .modules
            .libraries
            .debug
            .debug_library(world.handle.player(), "/lib/metaxploit.so"),
    )
    .unwrap()
    .unwrap()
    .zero_day()
    .address;
    let mut library = block_on(
        world
            .modules
            .libraries
            .debug
            .debug_library(world.handle.player(), "/lib/metaxploit.so"),
    )
    .unwrap()
    .unwrap();

    library.patch_seed = Some(library.seed - 1);
    block_on(world.modules.libraries.debug.save_debug_library(&library)).unwrap();

    let lines = format!("{:?}", library.challenge().error_lines);

    let code = format!(
        r#"
meta = include_lib("/lib/metaxploit.so")
lib = meta.load("/lib/metaxploit.so")
debug = lib.debug_tools("dbg", "secret")
print debug.unit_testing({lines})
objects = debug.payload("{address}")
print len(objects)
print objects[0]["classID"]
print objects[1]["classID"]
print objects[2]["classID"]
"#
    );

    let (output, error, restored) = run_on_host(&code, world, true);

    assert_eq!(error, None);
    assert!(
        output.contains("Starting testing in selected lines"),
        "{output}"
    );
    assert!(
        output.contains("A potential issue has been found"),
        "{output}"
    );
    assert!(output.contains("New vulnerability found"), "{output}");
    assert!(output.ends_with("3\npcomputer\npfile\nMetaLib"), "{output}");
    assert!(
        !block_on(
            restored
                .modules
                .libraries
                .debug
                .debug_library(restored.player(), "/lib/metaxploit.so")
        )
        .unwrap()
        .unwrap()
        .is_patched(restored.time_seconds())
    );
}

#[test]
fn should_reject_payload_for_an_unknown_memory_zone() {
    let world = debug_world();
    let code = r#"
meta = include_lib("/lib/metaxploit.so")
lib = meta.load("/lib/metaxploit.so")
debug = lib.debug_tools("dbg", "secret")
print debug.payload("0xBAD")
"#;

    let (output, error, _) = run_on_host(code, world, true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "Failed: No vulnerabilities have been found in the specified memory area."
    );
}

#[tokio::test]
async fn should_preserve_the_unit_test_lockout() {
    let mut world = debug_world();

    world
        .modules
        .libraries
        .debug
        .save_debug_account(&DebugAccount::new("test", "test"))
        .await
        .unwrap();

    let mut account = world
        .modules
        .libraries
        .debug
        .debug_account("test")
        .await
        .unwrap()
        .unwrap();

    account.unit_test_failed = true;
    world
        .modules
        .libraries
        .debug
        .save_debug_account(&account)
        .await
        .unwrap();

    let database = save_copy(&mut world).await;

    let restored = World::load(&database).await.unwrap();

    assert!(
        restored
            .modules
            .libraries
            .debug
            .debug_account("test")
            .await
            .unwrap()
            .unwrap()
            .unit_test_failed
    );
}

#[test]
fn should_generate_nonremote_zero_day_requirements_deterministically() {
    let world = debug_world();
    let library = block_on(
        world
            .modules
            .libraries
            .debug
            .debug_library(world.handle.player(), "/lib/metaxploit.so"),
    )
    .unwrap()
    .unwrap();
    let first = library.zero_day();
    let second = library.zero_day();

    assert_eq!(first, second);
    assert_eq!(first.vulnerability.requirements.len(), 3);
    for requirement in [
        Requirement::Library,
        Requirement::NumberUsersRegister,
        Requirement::PathExist,
    ] {
        assert!(first.vulnerability.requirements.contains(&requirement));
    }

    assert!(
        !first
            .vulnerability
            .requirements
            .contains(&Requirement::LocalNetwork)
    );
    assert_eq!(first.vulnerability.user, "guest");
}
