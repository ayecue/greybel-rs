use greybel_test_support::future::block_on;
use greyhack_world::{ComputerOptions, NetworkOptions, PlayerOptions, World};

use crate::support::network::literal;
use crate::support::scripts::run_on_host;

#[test]
fn should_change_and_read_an_npc_phone_alarm() {
    let mut world = block_on(World::new(&PlayerOptions::default())).unwrap();
    let router = block_on(world.add_network(&NetworkOptions::default())).unwrap();
    let public_ip = block_on(world.router(router)).unwrap().unwrap().public_ip;
    let mut npc = None;

    for computer in block_on(world.devices(router)).unwrap() {
        if let Some(found) = block_on(world.npcs_on(computer)).unwrap().first() {
            npc = Some(found.user.clone());
            break;
        }
    }

    let user = npc.expect("a generated NPC");
    let code = format!(
        r#"admin = include_lib("admin.so")
network = admin.network({})
print network.set_phone_alarm({}, 24, 0, 0)
alarm = network.set_phone_alarm({}, 7, 30, 0)
print [alarm.hour, alarm.minute, alarm.isAlarmModified, alarm.isAlarmDeleted, alarm.lateStrikes, alarm.fired]
print network.phone_alarm({}).hour
print network.set_phone_alarm({}, 0, 0, 0).fired"#,
        literal(&public_ip),
        literal(&user),
        literal(&user),
        literal(&user),
        literal(&user),
    );

    let (output, error, _) = run_on_host(&code, world, true);

    assert_eq!(error, None);
    assert!(
        output.starts_with(
            "Error: Incorrect time. A 24-hour format is expected.\n[7, 30, 1, 0, 0, 0]\n7\n0"
        ),
        "{output}"
    );
}

#[test]
fn should_report_a_library_update_for_the_network() {
    let mut world = block_on(World::new(&PlayerOptions::default())).unwrap();
    let router = block_on(world.add_network(&NetworkOptions {
        computers: Some(0),
        ..NetworkOptions::default()
    }))
    .unwrap();
    let public_ip = block_on(world.router(router)).unwrap().unwrap().public_ip;
    let options = ComputerOptions {
        services: Some(vec!["ssh".to_owned()]),
        ..ComputerOptions::default()
    };
    let mut computers = Vec::new();

    for version in ["1.0.0", "2.0.0"] {
        let computer = block_on(world.add_computer_to(router, &options))
            .unwrap()
            .unwrap();

        assert!(block_on(world.set_library_version(computer, "/lib/libssh.so", version)).unwrap());
        computers.push(computer);
    }

    let code = format!(
        r#"admin = include_lib("admin.so")
network = admin.network({})
print network.report_library_update("libssh.so")
print network.report_library_update("libnothing.so")"#,
        literal(&public_ip),
    );

    let (output, error, world) = run_on_host(&code, world, true);

    assert_eq!(error, None);
    assert_eq!(output.trim_end(), "1\n0");

    let library = block_on(world.file_library(computers[0], "/lib/libssh.so", false))
        .unwrap()
        .unwrap();

    assert_eq!(library.version, "2.0.0");
}

#[test]
fn should_skip_a_library_update_when_the_guild_turned_admin_restore_off() {
    let mut world = block_on(World::new(&PlayerOptions::default())).unwrap();
    let router = block_on(world.add_network(&NetworkOptions {
        computers: Some(0),
        ..NetworkOptions::default()
    }))
    .unwrap();
    let public_ip = block_on(world.router(router)).unwrap().unwrap().public_ip;
    let computer = block_on(world.add_computer_to(
        router,
        &ComputerOptions {
            services: Some(vec!["ssh".to_owned()]),
            ..ComputerOptions::default()
        },
    ))
    .unwrap()
    .unwrap();

    assert!(block_on(world.set_library_version(computer, "/lib/libssh.so", "1.0.0")).unwrap());

    let code = format!(
        r#"admin = include_lib("admin.so")
network = admin.network({})
print admin.create_guild("Owls", "owls-bank", 0, 17600, 0, 1)
print network.set_admin_restore("Owls", 0)
print network.set_company_shares("owls-bank", 2000000)
print network.set_admin_restore("Hawks", 0)
print network.set_admin_restore("Owls", 0)
print network.report_library_update("libssh.so")
print network.set_admin_restore("Owls", 1)
print network.report_library_update("libssh.so")"#,
        literal(&public_ip),
    );

    let (output, error, world) = run_on_host(&code, world, true);

    assert_eq!(error, None);
    assert_eq!(
        output.trim_end(),
        "1\nError: Admin Restore can only be configured for guild-owned companies.\n1\nError: no guild is called Hawks\n1\nThanks for the warning, but I am not going to update them for now.\n1\n1"
    );

    let library = block_on(world.file_library(computer, "/lib/libssh.so", false))
        .unwrap()
        .unwrap();

    assert_ne!(library.version, "1.0.0");
}
