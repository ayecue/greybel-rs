//! Tests that game methods called as globals, with the object first, act as the method: Grey Hack's interpreter finds
//! a name it has no variable for among its intrinsics (`TAC.Context.GetVar`), and a class's type map holds the same
//! intrinsics. The classes a fixture world can't reach are here; `fixtures/game/detached.src` has the others.

use greybel_interpreter::RuntimeErrorKind;

use crate::support::debug::debug_world;
use crate::support::economy::world_with_ether_created_by_test;
use crate::support::scripts::{run_code_in, run_on_host};
use crate::support::smart::world_with_appliance;
use crate::support::traffic::{TrafficWorld, world_with_traffic};

#[test]
fn should_call_coin_and_subwallet_methods_as_globals() {
    let world = world_with_ether_created_by_test();
    let code = r#"
chain = include_lib("/lib/blockchain.so")
wallet = create_wallet(chain, "test", "test")
coin = get_coin(chain, "Ether", "test", "test")
print [typeof(coin), get_cycle_mining(coin) == coin.get_cycle_mining, get_reward(coin) == coin.get_reward]
print create_subwallet(coin, "test", get_pin(wallet), "alice", "pw")
sub = get_subwallet(coin, "alice")
print [typeof(sub), get_user(sub), get_balance_subwallet(sub) == sub.get_balance]
print [@get_balance_subwallet == @sub.get_balance, @delete_subwallet == @sub.delete, @reset_password_coin == @coin.reset_password]
print delete_subwallet(sub)
print get_balance_subwallet(sub)
"#;

    let (output, error, _) = run_on_host(code, world, true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "[\"coin\", 1, 1]\n1\n[\"subwallet\", \"alice\", 1]\n[1, 1, 1]\n1\nget_balance: The account does not exist."
    );
}

#[test]
fn should_call_smart_appliance_methods_as_globals() {
    let code = r#"
device = include_lib("/lib/libsmartappliance.so")
print [model(device), model(device) == device.model, @set_alarm == @device.set_alarm]
print set_alarm(device, 1)
print override_settings(device, 100, 50)
"#;

    let (output, error, _) = run_on_host(code, world_with_appliance(), true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "[\"ACME/FR1\", 1, 1]\n1\noverride_settings: invalid power or temperature for this model. Check the device \
         manual."
    );
}

#[test]
fn should_call_traffic_methods_as_globals() {
    let TrafficWorld { world, plate, .. } = world_with_traffic();
    let code = format!(
        "device = include_lib(\"/lib/libtrafficnet.so\")\nprint get_credentials_info(device)\nprint \
         locate_vehicle(device, \"{plate}\", \"secret\")\nprint [@camera_link_system == @device.camera_link_system, \
         @locate_vehicle == @device.locate_vehicle]"
    );

    let (output, error, _) = run_on_host(&code, world, true);

    assert_eq!(error, None);
    assert_eq!(output, "Commissioner Alice alice\n1\n[1, 1]");
}

#[test]
fn should_call_debug_library_methods_as_globals() {
    let code = r#"
meta = include_lib("/lib/metaxploit.so")
lib = load(meta, "/lib/metaxploit.so")
print [is_patched(lib), is_patched(lib) == lib.is_patched]
debug = debug_tools(lib, "dbg", "secret")
print [typeof(debug), scan_debuglib(debug) == debug.scan, @scan_debuglib == @debug.scan, typeof(@scan)]
"#;

    let (output, error, _) = run_on_host(code, debug_world(), true);

    assert_eq!(error, None);
    assert_eq!(output, "[0, 1]\n[\"debugLibrary\", 1, 1, \"function\"]");
}

#[test]
fn should_crash_where_a_detached_method_dereferences_another_classs_missing_helper() {
    let programs = [
        "port_number(get_router)",
        "port_number(get_router.__isa)",
        "get_lan_ip(get_router)",
        "is_closed(get_router)",
        "admin = include_lib(\"admin.so\"); r = admin.create_network({\"public_ip\": \"8.8.8.8\", \"computers\": 0}); lan = r.local_ip[:-1]; admin.network(\"8.8.8.8\").add_computer({\"lan_ip\": lan + \"10\", \"services\": [\"ssh\"]}); port_info(get_shell, r.used_ports[0])",
        "aircrack(get_shell, \"/capture.cap\")",
        "airmon(get_shell, \"start\", \"eth0\")",
        "aireplay(get_shell, \"AA:BB\", \"wifi\", 1)",
        "smtp_user_list(get_shell, \"8.8.8.8\", 25)",
        "encrypt(get_shell, \"/tmp/x\", \"password\")",
        "decrypt(get_shell, \"/tmp/x\", \"password\")",
        "is_encrypted(get_shell, \"/tmp/x\")",
        "rshell_client(get_shell, \"8.8.8.8\", 1222, \"client\")",
        "rshell_server(get_shell)",
        "sniffer(get_shell)",
        "net_use(get_shell, \"8.8.8.8\", 22)",
        "meta = include_lib(\"/lib/metaxploit.so\"); lib = load(meta, \"/lib/init.so\"); scan(get_shell, lib)",
        "meta = include_lib(\"/lib/metaxploit.so\"); lib = load(meta, \"/lib/init.so\"); address = scan(meta, lib)[0]; scan_address(get_shell, lib, address)",
        "mail_login(\"test@test.com\", \"test\"); fetch(get_shell)",
        "mail_login(\"test@test.com\", \"test\"); read(get_shell, [])",
        "mail_login(\"test@test.com\", \"test\"); send(get_shell, \"other@test.com\", \"subject\", \"message\")",
        "mail_login(\"test@test.com\", \"test\"); delete_mail(get_shell, [])",
        "mail_login(\"test@test.com\", \"test\"); delete_all(get_shell)",
    ];

    for code in programs {
        let (_, error) = run_code_in(code, true);

        assert_eq!(
            error,
            Some(RuntimeErrorKind::GameCrash.to_string()),
            "{code}"
        );
    }
}

#[test]
fn should_scan_an_unknown_address_without_a_metaxploit_receiver() {
    let code = r#"
meta = include_lib("/lib/metaxploit.so")
lib = load(meta, "/lib/init.so")
print scan_address(get_shell, lib, "0x0")
"#;

    let (output, error) = run_code_in(code, true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "decompiling source...\nsearching unsecure values...\n"
    );
}

#[test]
fn should_use_the_mail_cooldown_before_dereferencing_a_detached_account() {
    let code = r#"
mail = mail_login("test@test.com", "test")
fetch(mail)
print fetch(get_shell)
print fetch({})
"#;

    let (output, error) = run_code_in(code, true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "MetaMail: Wait 2 seconds before using mail again.\nnull"
    );
}

#[test]
fn should_return_null_for_every_cross_class_wallet_coin_and_subwallet_lookup() {
    let world = world_with_ether_created_by_test();
    let code = r#"
chain = include_lib("/lib/blockchain.so")
wallet = create_wallet(chain, "test", "test")
coin = get_coin(chain, "Ether", "test", "test")
print create_subwallet(coin, "test", get_pin(wallet), "alice", "pw")
sub = get_subwallet(coin, "alice")
print [reset_password_coin(wallet, "newpass"), reset_password(coin, "newpass")]
print [get_balance(sub, "Ether"), get_balance_subwallet(wallet)]
print [delete(sub), delete(wallet), delete(coin)]
print [buy_coin(get_shell, "Ether", 1, 1, "alice"), sell_coin(get_shell, "Ether", 1, 1, "alice")]
"#;

    let (output, error, _) = run_on_host(code, world, true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "1\n[null, null]\n[null, null]\n[null, null, null]\n[null, null]"
    );
}

#[test]
fn should_crash_when_mail_delete_gets_a_subwallet_through_the_shared_delete_key() {
    let world = world_with_ether_created_by_test();
    let code = r#"
chain = include_lib("/lib/blockchain.so")
wallet = create_wallet(chain, "test", "test")
coin = get_coin(chain, "Ether", "test", "test")
create_subwallet(coin, "test", get_pin(wallet), "alice", "pw")
sub = get_subwallet(coin, "alice")
delete_mail(sub, [])
"#;

    let (_, error, _) = run_on_host(code, world, true);

    assert_eq!(error, Some(RuntimeErrorKind::GameCrash.to_string()));
}
