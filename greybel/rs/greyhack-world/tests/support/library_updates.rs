use greyhack_behaviour::LibraryKind;
use greyhack_world::{ComputerId, ComputerOptions, Library, NetworkOptions, PlayerOptions, World};

use super::router;

pub const SSH: &str = "libssh.so";
pub const SSH_PATH: &str = "/lib/libssh.so";
pub const FTP_PATH: &str = "/lib/libftp.so";

/// A world with the default player.
pub async fn world() -> World {
    World::new(&PlayerOptions::default()).await.unwrap()
}

/// A network with a computer running SSH and FTP for each of `versions`, which it has the library file libssh.so at,
/// and its public address.
pub async fn network_of(world: &mut World, versions: &[&str]) -> (String, Vec<ComputerId>) {
    let router_id = world
        .modules
        .creation
        .network_generation
        .add_network(&NetworkOptions {
            computers: Some(0),
            ..NetworkOptions::default()
        })
        .await
        .unwrap();
    let public_ip = router(world, router_id).await.public_ip;
    let mut computers = Vec::new();

    for version in versions {
        let computer = world
            .modules
            .creation
            .network_generation
            .add_computer_to(
                router_id,
                &ComputerOptions {
                    services: Some(vec!["ssh".to_owned(), "ftp".to_owned()]),
                    ..ComputerOptions::default()
                },
            )
            .await
            .unwrap()
            .expect("a computer");

        assert!(
            world
                .modules
                .core
                .computers
                .hardware
                .set_library_version(computer, "/lib/libssh.so", version)
                .await
                .unwrap()
        );
        computers.push(computer);
    }

    (public_ip, computers)
}

/// The version of the library file `path` of `computer`.
pub async fn version_of(world: &World, computer: ComputerId, path: &str) -> String {
    world
        .modules
        .libraries
        .generator
        .full_file_library(computer, path, false)
        .await
        .unwrap()
        .expect("a library file")
        .version
}

/// The version `computer`'s port for the service of libssh.so reports.
pub async fn ssh_port_version(world: &World, computer: ComputerId) -> String {
    world
        .modules
        .core
        .computers
        .processes
        .ports(computer)
        .await
        .unwrap()
        .into_iter()
        .find(|port| port.service == greyhack_behaviour::ServiceId::Ssh)
        .expect("an SSH port")
        .version
}

/// A library of `kind` at `version` with no weakness.
pub fn library_at(kind: LibraryKind, version: &str) -> Library {
    Library {
        kind,
        version: version.to_owned(),
        zones: Vec::new(),
    }
}
