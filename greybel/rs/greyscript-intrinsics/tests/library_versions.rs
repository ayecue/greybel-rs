//! Library versions released while the world runs (`VersionsControl`), what scripts see of them, and the failures of
//! `aptclient.check_upgrade` (`AptClientIntrinsics.check_upgrade`).

use greybel_test_support::future::block_on;

use crate::support::exploit_shop::shop_world;
use crate::support::library_versions::plain_world;
use crate::support::scripts::run_on_host;

#[test]
fn should_return_the_failures_of_check_upgrade_in_the_order_the_game_checks() {
    let code = r#"apt = include_lib("/lib/aptclient.so")
print apt.check_upgrade(home_dir + "/blockchain.so")
print apt.update
print apt.check_upgrade("")
print apt.check_upgrade(home_dir + "/blockchain.so")
print apt.check_upgrade(home_dir + "/missing.so")"#;

    let (output, error, _) = run_on_host(code, plain_world(), true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "/etc/apt/aptcache.bin not found. Launch apt with the update option to refresh the file\n\
         \n\
         package name can not be empty\n\
         blockchain.so does not exist in this filesystem\n\
         missing.so not found in the server"
    );
}

#[test]
fn should_return_the_missing_network_before_the_missing_cache() {
    let world = plain_world();
    let player = world.handle.player();
    let mut card =
        block_on(world.modules.core.networks.net_cards.net_cards(player)).unwrap()[0].clone();
    let world = world;

    card.active = false;
    block_on(
        world
            .modules
            .core
            .networks
            .net_cards
            .save_net_card(player, 0, &card),
    )
    .unwrap();

    let code = r#"apt = include_lib("/lib/aptclient.so")
print apt.check_upgrade(home_dir + "/blockchain.so")"#;
    let (output, error, _) = run_on_host(code, world, true);

    assert_eq!(error, None);
    assert_eq!(output, "apt_check_upgrade: No internet access.");
}

#[test]
fn should_offer_a_released_version_to_check_upgrade_and_install() {
    let code = r#"admin = get_greybel_util()
apt = include_lib("/lib/aptclient.so")
mx = include_lib("/lib/metaxploit.so")
print apt.update
print apt.check_upgrade("/lib/libssh.so")
print admin.release_library_version("libssh.so")
print apt.check_upgrade("/lib/libssh.so")
print apt.install("libssh.so", home_dir)
print mx.load(home_dir + "/libssh.so").version
print apt.check_upgrade(home_dir + "/libssh.so")"#;

    let (output, error, _) = run_on_host(code, plain_world(), true);

    assert_eq!(error, None);
    assert_eq!(output, "\n0\n1.0.1\n1\n1\n1.0.1\n0");
}

#[test]
fn should_patch_the_exploits_of_the_released_version_in_the_shop() {
    let code = r#"admin = get_greybel_util()
for offer in admin.exploit_shop("libssh.so", "9.8.7")
  print offer.description.indexOf("libssh.so >= v9.8.7") != null
end for
print admin.release_library_version("libssh.so")
for offer in admin.exploit_shop("libssh.so", "9.8.7")
  print offer.description.indexOf("libssh.so = v9.8.7") != null
end for
for offer in admin.exploit_shop("libssh.so", "9.8.8")
  print offer.description.indexOf("libssh.so >= v9.8.8") != null
end for"#;

    let (output, error, _) = run_on_host(code, shop_world(), true);

    assert_eq!(error, None);
    // The password exploit stays as it was: `VersionsControl.UpdateExploitDesc` never finds its overflow call.
    assert_eq!(output, "1\n1\n9.8.8\n1\n0\n1\n1");
}

#[test]
fn should_refuse_to_release_a_library_the_world_does_not_have() {
    let code = r#"admin = get_greybel_util()
print admin.release_library_version("libnothing.so")
print admin.release_library_version("libssh")
print admin.release_library_version(1)"#;

    let (output, error, _) = run_on_host(code, plain_world(), true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "Error: Unknown lib libnothing.so\nError: Unknown lib libssh\nrelease_library_version: Invalid arguments"
    );
}

#[test]
fn should_report_a_weakness_the_way_the_report_window_does() {
    let code = r#"admin = get_greybel_util()
print admin.report_exploit("libssh.so", "0xAB", "buffer")
print admin.report_exploit("libssh.so", "0xAB", "loop")
print admin.report_exploit("metaxploit.so", "0xAB", "buffer")
print admin.report_exploit("libnothing.so", "0xAB", "buffer")
print admin.report_exploit("libssh.so", "0xFF", "buffer")
print admin.report_exploit("libssh.so", "0xAB", "nothing")"#;

    let (output, error, _) = run_on_host(code, shop_world(), true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "1\n\
         Your previous report is still processing. Please wait a few minutes before sending a new report.\n\
         This library cannot be reported\n\
         Error: Unknown lib libnothing.so\n\
         This memory zone does not exist in the current version of the library\n\
         The unsecure value nothing does not exist in the memory zone 0xAB"
    );
}

#[test]
fn should_release_the_version_a_reported_weakness_is_patched_in_when_the_world_clock_passes_the_check()
 {
    let code = r#"admin = get_greybel_util()
print admin.report_exploit("libssh.so", "0xAB", "buffer")
admin.tick(12599)
print admin.exploit_shop("libssh.so", "9.8.8")
admin.tick(1)
print admin.exploit_shop("libssh.so", "9.8.8").len > 0"#;

    let (output, error, _) = run_on_host(code, shop_world(), true);

    assert_eq!(error, None);
    assert_eq!(output, "1\nNo results found\n1");
}
