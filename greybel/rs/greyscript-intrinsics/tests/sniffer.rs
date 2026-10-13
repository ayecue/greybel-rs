use duplex_core::{Channel, ChannelOptions};
use duplex_test_support::local::run_local;
use greybel_test_support::future::block_on;
use greybel_test_support::greyhack;
use greybel_test_support::interpreter::{constant_clock, snapshot_options};
use greybel_test_support::world::{change_computer, work_ends};
use greyhack_behaviour::{DeviceKind, ServiceId};
use greyhack_world::{Database, PlayerOptions, WorkResult, World};
use greyhack_world_assets::ssh_encode;
use greyhack_world_operations::WorldServer;
use greyscript_intrinsics::protocol::{
    IncludedLibrary, MetaxploitSniffer, PlayerIncludeLib, Program, StartProgram,
};

use crate::support::network::{SERVICE_SEED, literal};
use crate::support::scripts::run_both_on_host;
use crate::support::sniffer::remote_service;
use crate::support::{read_routers, world_options};

#[test]
fn should_capture_a_connection_through_the_sniffers_network() {
    // Arrange
    let mut world =
        block_on(World::generate(&world_options(SERVICE_SEED, 1))).expect("a generated world");
    let target = remote_service(&world);
    let player = world.player();
    let player_ip = format!(
        "{}/{}",
        block_on(world.public_ip(player)).unwrap(),
        block_on(world.computer(player)).unwrap().unwrap().local_ip
    );
    let sniffer = block_on(world.start_sniffer(player)).unwrap().unwrap();

    // Act
    block_on(world.connect_service(
        player,
        &target.address,
        target.port,
        &target.service,
        &target.user,
        &target.password,
        &[],
    ))
    .unwrap()
    .unwrap();

    // Assert
    let ends = work_ends(&mut world);

    assert_eq!(ends.len(), 1);
    assert_eq!(ends[0].id, sniffer);

    let WorkResult::Capture(packet) = &ends[0].result else {
        panic!("the sniffer captured a packet");
    };

    assert_eq!(packet.source, player_ip);
    assert_eq!(packet.destination, target.address);
    assert_eq!(packet.protocol, target.port.to_string());
}

#[test]
fn should_capture_a_failed_login() {
    // Arrange
    let mut world =
        block_on(World::generate(&world_options(SERVICE_SEED, 1))).expect("a generated world");
    let target = remote_service(&world);
    let player = world.player();
    let sniffer = block_on(world.start_sniffer(player)).unwrap().unwrap();

    // Act
    let result = block_on(world.connect_service(
        player,
        &target.address,
        target.port,
        &target.service,
        &target.user,
        "incorrect",
        &[],
    ))
    .unwrap();

    // Assert
    assert!(result.is_err());

    let ends = work_ends(&mut world);

    assert_eq!(ends.len(), 1);
    assert_eq!(ends[0].id, sniffer);
    assert!(matches!(ends[0].result, WorkResult::Capture(_)));
}

#[test]
fn should_capture_only_the_devices_on_a_packets_path() {
    // Arrange
    let mut world =
        block_on(World::generate(&world_options(SERVICE_SEED, 1))).expect("a generated world");
    let target = remote_service(&world);
    let target_computer = target.computer;
    let unrelated = *target
        .devices
        .iter()
        .find(|device| **device != target_computer)
        .expect("an unrelated device on the target LAN");
    let unrelated_sniffer = block_on(world.start_sniffer(unrelated)).unwrap().unwrap();

    // Act: the first packet goes to the target, not its sibling; the second reaches the target.
    block_on(world.connect_service(
        world.player(),
        &target.address,
        target.port,
        &target.service,
        &target.user,
        &target.password,
        &[],
    ))
    .unwrap()
    .unwrap();
    assert!(work_ends(&mut world).is_empty());

    let target_sniffer = block_on(world.start_sniffer(target_computer))
        .unwrap()
        .unwrap();

    block_on(world.connect_service(
        world.player(),
        &target.address,
        target.port,
        &target.service,
        &target.user,
        &target.password,
        &[],
    ))
    .unwrap()
    .unwrap();

    // Assert
    let ends = work_ends(&mut world);

    assert_eq!(ends.len(), 1);
    assert_eq!(ends[0].id, target_sniffer);
    assert_ne!(ends[0].id, unrelated_sniffer);
}

#[test]
fn should_capture_on_the_destination_router() {
    // Arrange
    let mut world =
        block_on(World::generate(&world_options(SERVICE_SEED, 1))).expect("a generated world");
    let target = remote_service(&world);
    let sniffer = block_on(world.start_sniffer(target.router.computer()))
        .unwrap()
        .unwrap();

    // Act
    block_on(world.connect_service(
        world.player(),
        &target.address,
        target.port,
        &target.service,
        &target.user,
        &target.password,
        &[],
    ))
    .unwrap()
    .unwrap();

    // Assert
    let ends = work_ends(&mut world);

    assert_eq!(ends.len(), 1);
    assert_eq!(ends[0].id, sniffer);
    assert!(matches!(ends[0].result, WorkResult::Capture(_)));
}

#[test]
fn should_capture_on_hub_siblings() {
    // Arrange
    let mut world =
        block_on(World::generate(&world_options(SERVICE_SEED, 1))).expect("a generated world");
    let target = remote_service(&world);
    let holder = block_on(world.computer(target.computer))
        .unwrap()
        .unwrap()
        .parent
        .expect("the destination's device");
    let peer = block_on(world.add_computer_to(
        greyhack_world::RouterId::of(holder),
        &greyhack_world::ComputerOptions::default(),
    ))
    .unwrap()
    .expect("a sibling on the destination's device");

    block_on(change_computer(&mut world, holder, |row| {
        row.kind = DeviceKind::Hub;
    }));

    let sniffer = block_on(world.start_sniffer(peer)).unwrap().unwrap();

    // Act
    block_on(world.connect_service(
        world.player(),
        &target.address,
        target.port,
        &target.service,
        &target.user,
        &target.password,
        &[],
    ))
    .unwrap()
    .unwrap();

    // Assert
    let ends = work_ends(&mut world);

    assert_eq!(ends.len(), 1);
    assert_eq!(ends[0].id, sniffer);
    assert!(matches!(ends[0].result, WorkResult::Capture(_)));
}

#[test]
fn should_keep_lan_endpoints_in_a_captured_login() {
    // Arrange
    let mut world =
        block_on(World::generate(&world_options(SERVICE_SEED, 1))).expect("a generated world");
    let target = remote_service(&world);
    let target_computer = target.computer;
    let target_ip = block_on(world.computer(target_computer))
        .unwrap()
        .unwrap()
        .local_ip;
    let source = *target
        .devices
        .iter()
        .find(|device| **device != target_computer)
        .expect("a source on the same LAN");
    let source_ip = format!(
        "{}/{}",
        block_on(world.public_ip(source)).unwrap(),
        block_on(world.computer(source)).unwrap().unwrap().local_ip
    );
    let sniffer = block_on(world.start_sniffer(source)).unwrap().unwrap();

    // Act
    block_on(world.connect_service(
        source,
        &target_ip,
        target.port,
        &target.service,
        &target.user,
        &target.password,
        &[],
    ))
    .unwrap()
    .unwrap();

    // Assert
    let ends = work_ends(&mut world);

    assert_eq!(ends.len(), 1);
    assert_eq!(ends[0].id, sniffer);

    let WorkResult::Capture(packet) = &ends[0].result else {
        panic!("the sniffer captured a packet");
    };

    assert_eq!(packet.source, source_ip);
    assert_eq!(packet.destination, target_ip);
    assert_eq!(packet.protocol, target.port.to_string());
}

#[test]
fn should_not_capture_a_connection_of_other_networks() {
    // Arrange
    let mut world =
        block_on(World::generate(&world_options(SERVICE_SEED, 2))).expect("a generated world");
    let target = remote_service(&world);
    let player = world.player();
    let other_devices = read_routers(&world)
        .into_iter()
        .filter(|router| router.id != target.router)
        .map(|router| block_on(world.devices(router.id)).unwrap())
        .find(|devices| !devices.contains(&player))
        .expect("a third network");

    block_on(world.start_sniffer(other_devices[0]))
        .unwrap()
        .unwrap();

    // Act
    block_on(world.connect_service(
        player,
        &target.address,
        target.port,
        &target.service,
        &target.user,
        &target.password,
        &[],
    ))
    .unwrap()
    .unwrap();

    // Assert
    assert!(work_ends(&mut world).is_empty());
}

#[test]
fn should_capture_the_clients_encoded_ssh_password_after_decode() {
    // Arrange: the remote SSH service prefixes the password with `enc_`, then decodes it successfully.
    let mut world =
        block_on(World::generate(&world_options(SERVICE_SEED, 1))).expect("a generated world");
    let target = remote_service(&world);
    let target_computer = target.computer;

    block_on(world.install_service(target_computer, ServiceId::Ssh, "1.0.0")).unwrap();

    let address = target.address;
    let password = target.password;
    let files = [
        (
            "/server/conf/sshd.conf",
            r#"{"encryption_enabled":true,"path_enc":"/server/encode.src","path_dec":"/server/decode.bin"}"#,
        ),
        (
            "/server/encode.src",
            "print(\">END;enc_\" + params[1] + \";1\")",
        ),
        (
            "/server/decode.bin",
            "print(\">END;\" + params[1][4:] + \";1\")",
        ),
    ];

    for (path, content) in files {
        let mut file = block_on(world.file(target_computer, path, false))
            .unwrap()
            .unwrap();

        file.data_mut().unwrap().content = content.to_owned();
        block_on(world.save_node(&file)).unwrap();
    }

    let connecting = format!(
        "shell = get_shell.connect_service({}, {}, {}, {})\nprint shell != null",
        literal(&address),
        target.port,
        literal(&target.user),
        literal(&password)
    );

    // Act: a protocol-level sniffer avoids blocking a second interpreter while the real client runs both SSH scripts.
    let (captured, connected, error) = run_local(async {
        let server = WorldServer::with_database(
            world,
            Database::in_memory().await.unwrap(),
            constant_clock(),
        )
        .await
        .unwrap();
        let sniffer =
            Channel::<Program, _>::new(&server.connect_on_tokio(), ChannelOptions::default());
        let user = PlayerOptions::default().user;
        let session = sniffer
            .send(&StartProgram {
                user: user.clone().into(),
                path: Some("/bin/sniffer".into()),
            })
            .unwrap()
            .await
            .unwrap();
        let library = sniffer
            .send(&PlayerIncludeLib {
                program: session.program,
                path: "/lib/metaxploit.so".into(),
            })
            .unwrap()
            .await
            .unwrap();
        let IncludedLibrary::Metaxploit(metaxploit) = library else {
            panic!("the player has metaxploit.so");
        };
        let request = sniffer
            .send(&MetaxploitSniffer {
                program: session.program,
                metaxploit,
                encode_source: false,
                program_path: "/bin/sniffer".into(),
            })
            .unwrap();
        let (lines, options) = snapshot_options();
        let (captured, error) = futures_util::future::join(
            request,
            greyhack::run_in(&server, &connecting, &user, true, options),
        )
        .await;

        (captured.unwrap(), lines.text(), error)
    });

    // Assert: `PlayerUtils.ConnectService` records encPass only after the client's encode/decode flow.
    assert_eq!(error, None);
    assert!(connected.ends_with("\n1"), "{connected}");

    let captured = captured.unwrap();

    assert!(captured.encrypted);
    assert_eq!(captured.password, format!("enc_{password}"), "{captured:?}");
}

#[test]
fn should_wait_for_a_packet_and_save_its_encoder_when_requested() {
    // Arrange
    let mut world =
        block_on(World::generate(&world_options(SERVICE_SEED, 1))).expect("a generated world");
    let target = remote_service(&world);
    let target_computer = target.computer;

    block_on(world.install_service(target_computer, ServiceId::Ssh, "1.0.0")).unwrap();

    let sniffing = "meta = include_lib(\"/lib/metaxploit.so\")\ncaptured = meta.sniffer(1)\nget_shell.host_computer.touch(\"/home/test\", \"captured\")\nprint captured.indexOf(\"Incoming data captured\") != null";
    let connecting = format!(
        "while not get_shell.host_computer.File(\"/home/test/captured\")\n  get_shell.connect_service({}, {}, {}, {}, {})\nend while\nprint \"done\"",
        literal(&target.address),
        target.port,
        literal(&target.user),
        literal(&target.password),
        literal(&target.service)
    );

    // Act
    let (captured, connected, world) = run_both_on_host(sniffing, &connecting, world);

    // Assert
    assert_eq!(captured, "1");
    assert_eq!(connected, "done");
    assert_eq!(
        block_on(world.content(world.player(), "/home/test/encode.src", false))
            .unwrap()
            .as_deref(),
        Some(ssh_encode())
    );
}
