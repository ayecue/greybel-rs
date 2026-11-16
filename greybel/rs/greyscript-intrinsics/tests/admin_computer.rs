use greybel_test_support::future::block_on;
use greyhack_behaviour::{DeviceKind, LibraryKind, ServiceId};
use greyhack_world::{PlayerOptions, World};
use greyhack_world_libraries::Library;
use serde_json::json;

use crate::support::network::literal;
use crate::support::scripts::run_on_host;

#[test]
fn should_reach_world_and_service_channels_through_admin_library() {
    let world = block_on(World::new(&PlayerOptions::default())).unwrap();
    let player = world.handle.player();
    let public_ip = block_on(world.modules.core.networks.topology.public_ip(player)).unwrap();
    let lan_ip = block_on(world.modules.core.networks.topology.computer(player))
        .unwrap()
        .unwrap()
        .local_ip;
    let password = block_on(world.modules.core.computers.users.users(player)).unwrap()[0]
        .password
        .clone();

    block_on(
        world
            .modules
            .services
            .installs
            .install_service(player, ServiceId::Ssh, "1.0.0"),
    )
    .unwrap();
    block_on(world.modules.services.installs.install_service(
        player,
        ServiceId::Criminals,
        "1.0.0",
    ))
    .unwrap();

    let file = block_on(
        world
            .modules
            .core
            .files
            .lookup
            .file(player, "/lib/libssh.so", false),
    )
    .unwrap()
    .unwrap();

    block_on(world.modules.libraries.generator.set_full_library_of(
        &file,
        Some(&Library {
            kind: LibraryKind::Ssh,
            version: "1.0.0".to_owned(),
            zones: Vec::new(),
        }),
    ))
    .unwrap();

    let mut database = block_on(world.modules.core.files.lookup.file(
        player,
        ServiceId::Criminals.config_path(),
        false,
    ))
    .unwrap()
    .unwrap();

    database.data_mut().unwrap().content = serde_json::to_string(&json!({
        "records": [{"kind": 2, "person_id": "person-1", "full_name": "Before"}]
    }))
    .unwrap();
    block_on(world.modules.core.files.changes.save_node(&database)).unwrap();

    let mut computer = block_on(world.modules.core.networks.topology.computer(player))
        .unwrap()
        .unwrap();

    computer.kind = DeviceKind::Camera;
    block_on(
        world
            .modules
            .core
            .networks
            .topology
            .save_computer(&computer),
    )
    .unwrap();

    let update = serde_json::to_string(&json!({
        "kind": 2, "person_id": "person-1", "full_name": "After"
    }))
    .unwrap();
    let code = format!(
        r#"admin = get_greybel_util()
computer = admin.network({}).computer({})
print computer.add_net_card(1, 1)
print computer.set_library_version("/lib/libssh.so", "9.8.7")
print computer.set_port_library_version(22, "8.7.6")
print admin.register_password("standalone") == md5("standalone")
print computer.service_records("criminals").indexOf("Before") != null
print computer.update_service_record("criminals", {}) == ""
print computer.service_records("criminals").indexOf("After") != null
print computer.access_cctv(null) == null
print computer.access_cctv({}).indexOf("local_ip") != null"#,
        literal(&public_ip),
        literal(&lan_ip),
        literal(&update),
        literal(&password),
    );

    let (output, error, _) = run_on_host(&code, world, true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        std::iter::repeat_n("1", 9).collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn should_install_smartphone_adb_through_admin_library() {
    let world = block_on(World::new(&PlayerOptions::default())).unwrap();
    let player = world.handle.player();
    let public_ip = block_on(world.modules.core.networks.topology.public_ip(player)).unwrap();
    let mut computer = block_on(world.modules.core.networks.topology.computer(player))
        .unwrap()
        .unwrap();

    computer.kind = DeviceKind::Smartphone;
    block_on(
        world
            .modules
            .core
            .networks
            .topology
            .save_computer(&computer),
    )
    .unwrap();

    let code = format!(
        "print get_greybel_util().network({}).computer({}).install_smartphone_adb",
        literal(&public_ip),
        literal(&computer.local_ip)
    );

    let (output, error, world) = run_on_host(&code, world, true);

    assert_eq!(error, None);
    assert_eq!(output, "1");
    assert!(
        block_on(
            world
                .modules
                .core
                .computers
                .processes
                .has_service(player, ServiceId::Adb)
        )
        .unwrap()
    );
}
