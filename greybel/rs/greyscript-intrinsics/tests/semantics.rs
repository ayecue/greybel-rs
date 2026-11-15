use std::rc::Rc;

use greybel_test_support::future::block_on;
use greyhack_behaviour::{FirewallRule, LanFailure};
use greyhack_world::{PlayerOptions, World};
use greyhack_world_core::{ConnectionFailure, LoginFailure};
use scheduler::ManualClock;

use crate::support::network::{SERVICE_SEED, first_remote_service, literal};
use crate::support::scripts::{run_code_in, run_game_with_io, run_on_host};
use crate::support::semantics::{PollStdin, world_with_player_router_at_origin};
use crate::support::{read_computer, read_router, read_routers, world_options};

#[test]
fn should_resolve_the_router_of_a_lan_device() {
    let code = "print get_router(\"192.168.0.1\") != null";
    let (output, error) = run_code_in(code, true);

    assert_eq!(error, None);
    assert_eq!(output, "1");
}

#[test]
fn should_generate_a_router_for_an_unknown_public_address_once() {
    let code = "print get_router(\"7.7.7.7\") != null\nprint get_router(\"7.7.7.7\") == get_router(\"7.7.7.7\")";
    let world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let (output, error, world) = run_on_host(code, world, true);

    assert_eq!(error, None);
    assert_eq!(output, "1\n1");
    assert!(
        block_on(world.modules.core.networks.routers.router_at("7.7.7.7"))
            .unwrap()
            .is_some()
    );
}

#[test]
fn should_reject_a_non_library_file_with_a_library_name() {
    let code = r#"
pc = get_shell.host_computer
pc.touch(home_dir, "crypto.so")
print include_lib(home_dir + "/crypto.so") == null
"#;
    let (output, error) = run_code_in(code, true);

    assert_eq!(error, None);
    assert_eq!(output, "1");
}

#[test]
fn should_resolve_a_registered_domain_with_or_without_www() {
    let code = r#"include_lib("admin.so").create_network({"public_ip": "5.5.5.5", "domain": "acme.com", "computers": 1})
print nslookup("acme.com")
print nslookup("www.acme.com")
print whois("5.5.5.5").split(char(10))[0]
print nslookup("nowhere.org")"#;
    let (output, error) = run_code_in(code, true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "5.5.5.5\n5.5.5.5\nDomain name: www.acme.com\nIP not found"
    );
}

#[test]
fn should_refuse_a_service_connection_matching_a_firewall_deny_rule() {
    let world =
        block_on(World::generate(&world_options(SERVICE_SEED, 1))).expect("a generated world");
    let target = first_remote_service(&world);

    block_on(
        world
            .modules
            .core
            .networks
            .router_config
            .clear_firewall(target.router),
    )
    .unwrap();
    block_on(world.modules.core.networks.router_config.add_firewall_rule(
        target.router,
        &FirewallRule {
            allow: false,
            port: Some(target.port),
            source: None,
            destination: None,
        },
    ))
    .unwrap();

    let player = world.handle.player();
    let result = block_on(world.modules.core.networks.connect.connect_service(
        player,
        &target.address,
        target.port,
        &target.service,
        &target.user,
        &target.password,
        &[],
    ))
    .unwrap();

    assert_eq!(
        result,
        Err(LoginFailure::Connection(ConnectionFailure::Lan(
            LanFailure::Firewall
        )))
    );
}

#[test]
fn should_allow_a_service_connection_when_an_allow_rule_overrides_the_deny() {
    let world =
        block_on(World::generate(&world_options(SERVICE_SEED, 1))).expect("a generated world");
    let target = first_remote_service(&world);

    block_on(
        world
            .modules
            .core
            .networks
            .router_config
            .clear_firewall(target.router),
    )
    .unwrap();
    for allow in [false, true] {
        block_on(world.modules.core.networks.router_config.add_firewall_rule(
            target.router,
            &FirewallRule {
                allow,
                port: Some(target.port),
                source: None,
                destination: None,
            },
        ))
        .unwrap();
    }

    let player = world.handle.player();
    let result = block_on(world.modules.core.networks.connect.connect_service(
        player,
        &target.address,
        target.port,
        &target.service,
        &target.user,
        &target.password,
        &[],
    ))
    .unwrap();

    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn should_resolve_imports_when_building_a_program() {
    let code = r#"
sh = get_shell
pc = sh.host_computer
pc.touch(home_dir, "lib.src")
pc.File(home_dir + "/lib.src").set_content("x = 42")
pc.touch(home_dir, "main.src")
pc.File(home_dir + "/main.src").set_content("import_code(""/home/test/lib.src"")" + char(10) + "print(x)")
print "build=" + sh.build(home_dir + "/main.src", home_dir)
print "launch=" + sh.launch(home_dir + "/main")
"#;
    let world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let (output, error, _) = run_on_host(code, world, true);

    assert_eq!(error, None);
    assert_eq!(output, "build=\n42\nlaunch=1");
}

#[test]
fn should_reset_processes_on_reboot() {
    let code = "print get_shell(\"root\", \"test\").host_computer.reboot";
    let world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let (output, error, world) = run_on_host(code, world, true);

    assert_eq!(error, None);
    assert_eq!(output, "1");

    // The game keeps the player's protected system processes (`kernel_task`, `Xorg`) across a reboot.
    let processes = block_on(
        world
            .modules
            .core
            .computers
            .processes
            .processes(world.handle.player()),
    )
    .unwrap();

    assert!(!processes.is_empty());
    assert!(processes.iter().all(|process| process.protected));
}

#[test]
fn should_deny_reboot_to_a_user_outside_the_root_group() {
    let code = "get_shell(\"root\", \"test\").host_computer.create_user(\"other\", \"pw\")\nprint get_shell(\"other\", \"pw\").host_computer.reboot";
    let (output, error) = run_code_in(code, true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "Permission denied. Only root or users in the root group can execute this command."
    );
}

#[test]
fn should_list_wifi_networks_with_their_names() {
    let code = "print get_shell.host_computer.wifi_networks(\"wlan0\")";
    let (output, error) = run_code_in(code, true);

    assert_eq!(error, None);
    assert!(output.contains("test_net"), "{output}");
}

#[test]
fn should_connect_a_wifi_card_to_a_router() {
    let world = block_on(World::generate(&world_options(7, 1))).expect("a generated world");
    let router = read_routers(&world).swap_remove(1);
    let code = format!(
        "print get_shell.host_computer.connect_wifi(\"wlan0\", {}, {}, {})",
        literal(router.bssid.as_deref().unwrap_or_default()),
        literal(router.essid.as_deref().unwrap_or_default()),
        literal(&router.wifi_password)
    );
    let router_id = router.id;
    let (output, error, world) = run_on_host(&code, world, true);

    assert_eq!(error, None);
    assert_eq!(output, "1");

    let player = read_computer(&world, world.handle.player());

    assert_eq!(player.router, Some(router_id));
    assert!(
        block_on(
            world
                .modules
                .core
                .networks
                .net_cards
                .active_net_card(player.id)
        )
        .unwrap()
        .unwrap()
        .wifi
    );
}

#[test]
fn should_only_list_routers_within_wifi_range() {
    let world = world_with_player_router_at_origin();
    let player = world.handle.player();
    let near = block_on(world.modules.creation.network_generation.add_network(
        &greyhack_world_creation::NetworkOptions {
            public_ip: Some("8.8.8.8".to_owned()),
            position: Some([0.4, 0.0]),
            ..Default::default()
        },
    ))
    .unwrap();
    let far = block_on(world.modules.creation.network_generation.add_network(
        &greyhack_world_creation::NetworkOptions {
            public_ip: Some("9.9.9.9".to_owned()),
            position: Some([0.6, 0.0]),
            ..Default::default()
        },
    ))
    .unwrap();

    let networks = block_on(world.modules.core.networks.net_cards.wifi_networks(player)).unwrap();

    assert_eq!(networks.len(), 2);
    assert_eq!(networks[1].1, 20);
    assert!(
        networks
            .iter()
            .any(|(bssid, _, _)| Some(bssid) == read_router(&world, near).bssid.as_ref())
    );
    assert!(
        !networks
            .iter()
            .any(|(bssid, _, _)| Some(bssid) == read_router(&world, far).bssid.as_ref())
    );
}

#[test]
fn should_reject_a_wifi_router_out_of_range() {
    let world = world_with_player_router_at_origin();
    let router = block_on(world.modules.creation.network_generation.add_network(
        &greyhack_world_creation::NetworkOptions {
            public_ip: Some("9.9.9.9".to_owned()),
            position: Some([0.6, 0.0]),
            essid: Some("far".to_owned()),
            wifi_password: Some("pw".to_owned()),
            ..Default::default()
        },
    ))
    .unwrap();
    let router = read_router(&world, router);
    let code = format!(
        "print get_shell.host_computer.connect_wifi(\"wlan0\", {}, {}, \"pw\")",
        literal(router.bssid.as_deref().unwrap_or_default()),
        literal(router.essid.as_deref().unwrap_or_default())
    );

    let (output, error, _) = run_on_host(&code, world, true);

    assert_eq!(error, None);
    assert_eq!(output, "Can't connect. Target is out of reach.");
}

#[test]
fn should_throttle_wifi_reconnection() {
    let world = world_with_player_router_at_origin();
    let router = block_on(world.modules.creation.network_generation.add_network(
        &greyhack_world_creation::NetworkOptions {
            public_ip: Some("8.8.8.8".to_owned()),
            position: Some([0.1, 0.0]),
            wifi_password: Some("pw".to_owned()),
            ..Default::default()
        },
    ))
    .unwrap();
    let router = read_router(&world, router);
    let code = format!(
        "c = get_shell.host_computer\nprint c.connect_wifi(\"wlan0\", {}, {}, \"pw\")\nprint c.connect_wifi(\"wlan0\", {}, {}, \"pw\")\nwait(1.1)\nprint c.connect_wifi(\"wlan0\", {}, {}, \"pw\")",
        literal(router.bssid.as_deref().unwrap_or_default()),
        literal(router.essid.as_deref().unwrap_or_default()),
        literal(router.bssid.as_deref().unwrap_or_default()),
        literal(router.essid.as_deref().unwrap_or_default()),
        literal(router.bssid.as_deref().unwrap_or_default()),
        literal(router.essid.as_deref().unwrap_or_default())
    );
    let clock = Rc::new(ManualClock::advance_on_sleep());

    let (output, error, _) = run_game_with_io(&code, world, true, Rc::new(PollStdin), clock);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "1\nconnect_wifi: You must wait a moment before reconnecting to another network.\n1"
    );
}

#[test]
fn should_throttle_poll_input_to_one_key_per_interval() {
    let world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let clock = Rc::new(ManualClock::advance_on_sleep());
    let code = "print poll_input\nwait(0.3)\nprint poll_input\nprint poll_input";

    let (output, error, _) = run_game_with_io(code, world, true, Rc::new(PollStdin), clock);

    assert_eq!(error, None);
    assert_eq!(output, "a\na\n");
}

#[test]
fn should_return_an_empty_string_when_poll_input_has_no_key() {
    let code = "print poll_input == \"\"";

    let (output, error) = run_code_in(code, true);

    assert_eq!(error, None);
    assert_eq!(output, "1");
}

#[test]
fn should_return_the_same_custom_object_for_each_call() {
    let code = "print get_custom_object.classID\na = get_custom_object\na[\"x\"] = 42\nprint get_custom_object.x\nprint get_custom_object.__isa.len";

    let (output, error) = run_code_in(code, true);

    assert_eq!(error, None);
    assert_eq!(output, "custom_object\n42\n0");
}

#[test]
fn should_connect_an_ethernet_card_with_a_valid_address() {
    let world = block_on(World::generate(&world_options(7, 1))).expect("a generated world");
    let router = read_routers(&world).swap_remove(1);
    let gateway = router.local_ip.clone();
    let address = format!("{}.20", gateway.rsplit_once('.').unwrap().0);
    let router_id = router.id;
    let computer = block_on(world.modules.core.networks.topology.devices(router_id))
        .unwrap()
        .into_iter()
        .find(|computer| {
            block_on(world.modules.core.networks.net_cards.net_cards(*computer))
                .unwrap()
                .iter()
                .any(|card| !card.wifi)
        })
        .expect("a computer with an Ethernet card");

    let result = block_on(
        world
            .modules
            .core
            .networks
            .net_cards
            .connect_ethernet(computer, &address, &gateway),
    )
    .unwrap();

    assert_eq!(result, Ok(()));

    let computer_row = read_computer(&world, computer);

    assert_eq!(computer_row.router, Some(router_id));
    assert_eq!(computer_row.local_ip, address);
    assert!(
        !block_on(
            world
                .modules
                .core
                .networks
                .net_cards
                .active_net_card(computer)
        )
        .unwrap()
        .unwrap()
        .wifi
    );
}

#[test]
fn should_write_file_content_up_to_the_game_limit() {
    let code = r#"
pc = get_shell.host_computer
pc.touch(home_dir, "big.txt")
file = pc.File(home_dir + "/big.txt")
content = "a" * 160000
print file.set_content(content)
print file.get_content.len
print file.set_content(content + "a")
"#;
    let world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let (output, error, _) = run_on_host(code, world, true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "1\n160000\nI can't save the file. The maximum of 160,000 characters has been exceeded."
    );
}

#[test]
fn should_name_the_import_a_crashed_program_failed_in() {
    let code = r#"
sh = get_shell
pc = sh.host_computer
write = function(name, content)
    pc.touch(home_dir, name)
    pc.File(home_dir + "/" + name).set_content(content)
end function
write("lib.src", "a = 1" + char(10) + "b = [1][5]")
write("inner.src", "c = 1" + char(10) + "d = 2")
write("main.src", "print 1" + char(10) + "import_code(""/home/test/lib.src"")" + char(10) + "print 2")
write("after.src", "print 1" + char(10) + "import_code(""/home/test/inner.src"")" + char(10) + "x = [1][5]")
write("plain.src", "print 1" + char(10) + "x = [1][5]")
for name in ["main", "after", "plain"]
    print "build=" + sh.build(home_dir + "/" + name + ".src", home_dir)
    sh.launch(home_dir + "/" + name)
end for
"#;
    let world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let (output, error, _) = run_on_host(code, world, true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "build=\n1\n<color=red>Runtime Error: Index Error: list index (5) out of range (-1 to 0) [lib.src line 2]</color>\n\
         build=\n1\n<color=red>Runtime Error: Index Error: list index (5) out of range (-1 to 0) [line 3]</color>\n\
         build=\n1\n<color=red>Runtime Error: Index Error: list index (5) out of range (-1 to 0) [/home/test/plain line 2]</color>"
    );
}

#[test]
fn should_leave_the_imports_of_an_imported_file_in_its_code() {
    let code = r#"
sh = get_shell
pc = sh.host_computer
pc.touch(home_dir, "deep.src")
pc.File(home_dir + "/deep.src").set_content("y = 1")
pc.touch(home_dir, "lib.src")
pc.File(home_dir + "/lib.src").set_content("import_code(""/home/test/deep.src"")")
pc.touch(home_dir, "main.src")
pc.File(home_dir + "/main.src").set_content("import_code(""/home/test/lib.src"")" + char(10) + "print(y)")
print "build=" + sh.build(home_dir + "/main.src", home_dir)
sh.launch(home_dir + "/main")
"#;
    let world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let (output, error, _) = run_on_host(code, world, true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "build=\n<color=red>Runtime Error: Undefined Identifier: 'import_code' is unknown in this context [lib.src line 1]</color>"
    );
}

#[test]
fn should_crash_the_script_on_an_import_without_a_closing_parenthesis() {
    let code = r#"
sh = get_shell
pc = sh.host_computer
pc.touch(home_dir, "main.src")
pc.File(home_dir + "/main.src").set_content("import_code(""/home/test/lib.src""")
print "build=" + sh.build(home_dir + "/main.src", home_dir)
"#;
    let world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let (output, error, _) = run_on_host(code, world, true);

    assert_eq!(output, "");
    assert_eq!(
        error.map(|error| error.to_string()),
        Some(greybel_interpreter::RuntimeErrorKind::GameCrash.to_string())
    );
}

/// The game's `Math.Sign` throws for NaN, which crashes the script.
#[test]
fn should_crash_the_script_on_the_sign_of_nan() {
    let world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let (output, error, _) = run_on_host("print sign(-2); print sign(0 / 0)", world, true);

    assert_eq!(output, "-1");
    assert_eq!(
        error.map(|error| error.to_string()),
        Some(greybel_interpreter::RuntimeErrorKind::GameCrash.to_string())
    );
}

/// Each output as Grey Hack prints it for the same script.
#[test]
fn should_launch_a_program_from_itself() {
    let code = r#"
sh = get_shell
pc = sh.host_computer
write = function(name, lines)
    pc.touch(home_dir, name + ".src")
    pc.File(home_dir + "/" + name + ".src").set_content(lines.join(char(10)))
    sh.build(home_dir + "/" + name + ".src", home_dir)
end function
write("nested", [
    "n = params[0].to_int",
    "print ""in "" + n + "" "" + program_path",
    "if n == 2 then x = [1][5]",
    "if n < 4 then print ""r"" + n + ""="" + get_shell.launch(program_path, str(n + 1))",
    "print ""out "" + n"])
write("endless", [
    "n = params[0].to_int",
    "if not get_shell.launch(program_path, str(n + 1)) then print ""refused at "" + n"])
write("shared", [
    "c = get_custom_object",
    "if not c.hasIndex(""n"") then c.n = 0",
    "c.n = c.n + 1",
    "if c.n < 4 then get_shell.launch(program_path)",
    "print ""n="" + c.n"])
print "top=" + sh.launch(home_dir + "/nested", "0")
print "top=" + sh.launch(home_dir + "/endless", "0")
print "top=" + sh.launch(home_dir + "/shared")
"#;
    let world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let (output, error, _) = run_on_host(code, world, true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "in 0 /home/test/nested\nin 1 /home/test/nested\nin 2 /home/test/nested\n\
         <color=red>Runtime Error: Index Error: list index (5) out of range (-1 to 0) [/home/test/nested line 3]</color>\n\
         r1=1\nout 1\nr0=1\nout 0\ntop=1\n\
         Program interrupted. Too many stack calls.\nrefused at 32\ntop=1\n\
         n=4\nn=4\nn=4\nn=4\ntop=1"
    );
}
