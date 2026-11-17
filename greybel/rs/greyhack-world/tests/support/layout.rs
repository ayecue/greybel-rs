use greyhack_behaviour::{
    DeviceKind, FileData, FileKind, LibraryKind, NetworkKind, PermissionSet, ROOT_USER, Rng,
    STARTUP_FILES, ServiceId, StartupContent,
};
use greyhack_world_core::{Port, User};
use greyhack_world_creation::{
    DeviceLayout, ForwardLayout, LanNodeLayout, NetworkLayout, router_computer,
};
use greyhack_world_libraries::{ComputerDraft, Library, NodeDraft, RouterDraft};

fn router(kind: DeviceKind, local_ip: &str) -> RouterDraft {
    let kernel = Library {
        kind: LibraryKind::KernelRouter,
        version: "1.0.0".to_owned(),
        zones: Vec::new(),
    };

    RouterDraft {
        essid: None,
        bssid: None,
        wifi_password: "wifi".to_owned(),
        firewall_rules: Vec::new(),
        firewall_active: true,
        computer: Box::new(router_computer(
            &mut Rng::new(1),
            kind,
            local_ip,
            "root",
            kernel,
            "1.0.0",
        )),
    }
}

/// Gives `computer` the files its startup checks need (`Computer.CheckInicio`).
pub fn startup_files(computer: &mut ComputerDraft) {
    let filesystem = &mut computer.filesystem;
    let root = filesystem.root();

    for (path, _) in STARTUP_FILES {
        let (folder, name) = path.rsplit_once('/').expect("an absolute path");
        let parent = filesystem.folder(folder, false).unwrap_or_else(|| {
            filesystem.add(
                root,
                NodeDraft::folder(
                    folder.trim_start_matches('/'),
                    ROOT_USER,
                    PermissionSet::default_for(true),
                ),
            )
        });
        let permissions = PermissionSet::default_for(false);
        let node = match StartupContent::of(path) {
            StartupContent::Library => {
                let data = FileData {
                    kind: FileKind::Library,
                    ..FileData::default()
                };
                let mut node = NodeDraft::file(name, ROOT_USER, permissions, data);

                node.library = Some(Box::new(Library {
                    kind: LibraryKind::of_file(name).expect("a library"),
                    version: "1.0.0".to_owned(),
                    zones: Vec::new(),
                }));

                node
            }
            StartupContent::Text => {
                NodeDraft::file(name, ROOT_USER, permissions, FileData::text(""))
            }
            StartupContent::Binary => {
                let data = FileData {
                    kind: FileKind::Binary,
                    ..FileData::default()
                };

                NodeDraft::file(name, ROOT_USER, permissions, data)
            }
        };

        filesystem.add(parent, node);
    }
}

fn computer(name: &str, local_ip: &str, player: bool) -> Box<ComputerDraft> {
    Box::new(ComputerDraft {
        is_player: player,
        users: vec![User::new("root", "root"), User::new("alice", "secret")],
        ..ComputerDraft::new(name, local_ip)
    })
}

/// A network like the game's test network: the main router 192.168.1.1 with the player's computer and one forwarding
/// SSH to a computer behind the LAN router 172.16.3.1, a hub under it with one more computer.
pub fn layout() -> NetworkLayout {
    let mut server = computer("server", "172.16.3.7", false);

    server.ports.push(Port {
        number: 22,
        service: ServiceId::Ssh,
        version: "1.0.0".to_owned(),
        closed: false,
        visible: true,
    });

    let node = |parent: Option<usize>, gateway: &str, device: DeviceLayout| LanNodeLayout {
        parent,
        gateway: gateway.to_owned(),
        device,
    };

    NetworkLayout {
        kind: NetworkKind::Residential,
        public_ip: "45.45.45.45".to_owned(),
        position: greyhack_behaviour::position_for_ip("45.45.45.45"),
        domain: String::new(),
        contact: None,
        admin: None,
        disabled: false,
        net_speed: greyhack_behaviour::DEFAULT_NET_SPEED,
        rental_admin: false,
        library_choices: Vec::new(),
        nodes: vec![
            node(
                None,
                "",
                DeviceLayout::Router {
                    draft: router(DeviceKind::Router, "192.168.1.1"),
                    ports: Vec::new(),
                    forwards: vec![ForwardLayout {
                        number: 2222,
                        lan_ip: "172.16.3.7".to_owned(),
                        port: 22,
                    }],
                },
            ),
            node(
                Some(0),
                "192.168.1.1",
                DeviceLayout::Computer(computer("test", "192.168.1.6", true)),
            ),
            node(
                Some(0),
                "",
                DeviceLayout::Router {
                    draft: router(DeviceKind::Router, "172.16.3.1"),
                    ports: Vec::new(),
                    forwards: Vec::new(),
                },
            ),
            node(Some(2), "172.16.3.1", DeviceLayout::Computer(server)),
            node(Some(2), "", DeviceLayout::Hub),
            node(
                Some(4),
                "172.16.3.1",
                DeviceLayout::Computer(computer("hub pc", "172.16.3.9", false)),
            ),
        ],
    }
}
