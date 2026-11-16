use greybel_test_support::future::block_on;
use greyhack_behaviour::JobRole;

use crate::support::guilds::{add_company, crown};
use crate::support::network::literal;
use crate::support::scripts::run_on_host;
use crate::support::traffic::{TrafficWorld, world_with_traffic};

#[test]
fn should_create_guilds_and_hand_out_shares_from_a_script() {
    let TrafficWorld { mut world, .. } = world_with_traffic();
    let address = add_company(&mut world);
    let code = format!(
        r#"admin = get_greybel_util()
print admin.create_guild("Owls", "owls-bank", 1, 6875, 1)
print admin.create_guild("Owls", "other", 0, 0, 0)
print admin.create_guild("Hawks", "owls-bank", 0, 0, 0)
network = admin.network({})
print network.set_company_shares("owls-bank", 100000)
print network.hire_crown_shield("Owls")
print network.hire_crown_shield("Nobody")"#,
        literal(&address),
    );

    let (output, error, _) = run_on_host(&code, world, true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "1\nError: a guild is called Owls already\nError: a guild holds shares under owls-bank already\n1\nCrown required\nError: no guild is called Nobody"
    );
}

#[test]
fn should_refuse_to_locate_the_vehicle_of_an_administrator_under_crownshield() {
    let TrafficWorld {
        mut world,
        plate,
        driver,
        ..
    } = world_with_traffic();
    let address = add_company(&mut world);
    let home = block_on(
        world
            .modules
            .core
            .networks
            .topology
            .public_ip(world.handle.player()),
    )
    .unwrap();

    block_on(
        world
            .modules
            .npcs
            .jobs
            .set_npc_job(driver, JobRole::Admin, Some(plate.clone())),
    )
    .unwrap();
    crown(&mut world, &address);
    block_on(world.modules.guilds.guilds.hire_crown_shield("Owls", &home))
        .unwrap()
        .unwrap();
    block_on(world.tick(600)).unwrap();
    assert!(block_on(world.modules.guilds.guilds.is_crown_shield_protected(&home)).unwrap());

    let code = format!(
        "device = include_lib(\"/lib/libtrafficnet.so\")\nprint device.locate_vehicle(\"{plate}\", \"secret\")"
    );

    let (output, error, _) = run_on_host(&code, world, true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "The vehicle could not be located: CrownShield provides secure transport for this network administrator."
    );
}

#[test]
fn should_interrupt_an_overflow_against_a_network_under_crownshield() {
    use crate::support::system_log::{ADDRESS, UNSEC_VALUE, public_ip, world_with_shell_weakness};

    let (mut world, target) = world_with_shell_weakness();
    let company = add_company(&mut world);
    let target_ip = public_ip(&world, target.computer);
    let script = format!(
        "mx = include_lib(\"/lib/metaxploit.so\")\nsession = mx.net_use(\"{}\", {})\nlib = session.dump_lib\nshell = lib.overflow(\"{ADDRESS}\", \"{UNSEC_VALUE}\")\nprint typeof(shell)\nmissing = lib.overflow(\"0x0\", \"nothing\")\nprint typeof(missing)",
        target.address, target.port
    );

    crown(&mut world, &company);
    block_on(
        world
            .modules
            .guilds
            .guilds
            .hire_crown_shield("Owls", &target_ip),
    )
    .unwrap()
    .unwrap();
    block_on(world.tick(600)).unwrap();

    let (output, error, _) = run_on_host(&script, world, true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "Attack interrupted: this network is protected by CrownShield.\nnull\nExploit failed. Vulnerability not found.\nnull"
    );
}
