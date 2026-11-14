use greyhack_world::World;

use crate::support::debug::debug_world;
use crate::support::save_copy;
use crate::support::scripts::run_on_host;

#[test]
fn should_log_into_debug_tools_and_apply_a_patch() {
    let world = debug_world();
    let code = r#"
meta = include_lib("/lib/metaxploit.so")
lib = meta.load("/lib/metaxploit.so")
print lib.is_patched
print lib.is_patched(1)
debug = lib.debug_tools("dbg", "secret")
print debug.scan
print debug.apply_patch("/root/patch.src")
print lib.is_patched
print debug.scan
"#;

    let (output, error, _) = run_on_host(code, world, true);

    assert_eq!(error, None);
    assert!(output.starts_with("0\n0\n"), "{output}");
    assert!(output.contains("The function func_"), "{output}");
    assert!(
        output.contains("The patch has been applied correctly."),
        "{output}"
    );
    assert!(
        output.ends_with("1\nNo potential issues have been found"),
        "{output}"
    );
}

#[test]
fn should_reject_bad_debug_credentials() {
    let world = debug_world();
    let code = r#"
meta = include_lib("/lib/metaxploit.so")
lib = meta.load("/lib/metaxploit.so")
print lib.debug_tools("dbg", "bad")
"#;

    let (output, error, _) = run_on_host(code, world, true);

    assert_eq!(error, None);
    assert_eq!(output, "Error: Incorrect password");
}

#[tokio::test]
async fn should_preserve_debug_accounts_and_patches() {
    let mut world = debug_world();
    let mut library = world
        .modules
        .libraries
        .debug
        .debug_library(world.handle.player(), "/lib/metaxploit.so")
        .await
        .unwrap()
        .unwrap();

    library.patch_seed = Some(library.seed);
    world
        .modules
        .libraries
        .debug
        .save_debug_library(&library)
        .await
        .unwrap();

    let database = save_copy(&mut world).await;

    let restored = World::load(&database).await.unwrap();

    assert!(
        restored
            .modules
            .libraries
            .debug
            .debug_account("dbg")
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        restored
            .modules
            .libraries
            .debug
            .debug_library(restored.player(), "/lib/metaxploit.so")
            .await
            .unwrap()
            .unwrap()
            .is_patched(restored.time_seconds())
    );
}
