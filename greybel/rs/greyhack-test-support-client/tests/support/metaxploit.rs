//! Scripts built from the weaknesses of libraries mirrored from the game.

use greyhack_behaviour::Reward;
use greyhack_test_support_client::protocol::test_support::{
    ComputerSnapshot, LanNodeKind, NetworkSnapshot,
};
use greyhack_world::{Library, Weakness};

use super::mirror::library;

/// The player computer in `snapshot`.
pub fn player(snapshot: &NetworkSnapshot) -> &ComputerSnapshot {
    snapshot
        .nodes
        .iter()
        .find_map(|node| match &node.node {
            LanNodeKind::Computer(computer) if computer.player => Some(computer),
            _ => None,
        })
        .expect("the snapshot has the player computer")
}

/// Every library file on the player computer.
pub fn player_libraries(snapshot: &NetworkSnapshot) -> Vec<(String, Library)> {
    player(snapshot)
        .files
        .iter()
        .filter_map(|file| Some((file.path.to_string(), library(&file.content)?)))
        .collect()
}

/// The library `name` of the computer at `local_ip`.
pub fn computer_library(snapshot: &NetworkSnapshot, local_ip: &str, name: &str) -> Library {
    snapshot
        .nodes
        .iter()
        .find_map(|node| {
            if node.local_ip != local_ip {
                return None;
            }

            let LanNodeKind::Computer(computer) = &node.node else {
                return None;
            };
            let path = format!("/lib/{name}");

            computer
                .files
                .iter()
                .find(|file| file.path == path)
                .and_then(|file| library(&file.content))
        })
        .expect("the mirrored computer has the library")
}

/// The kernel library of the router or switch at `local_ip`.
pub fn router_library(snapshot: &NetworkSnapshot, local_ip: &str) -> Library {
    snapshot
        .nodes
        .iter()
        .find_map(|node| {
            if node.local_ip != local_ip {
                return None;
            }

            let LanNodeKind::Router(router) = &node.node else {
                return None;
            };

            Some(library(&router.kernel_content).unwrap_or_else(|| {
                panic!(
                    "the mirrored router at {local_ip} has no kernel library in {:?}",
                    router.kernel_content
                )
            }))
        })
        .expect("the snapshot has the router")
}

/// The kernel libraries the generated network actually gave its routers and switches.
pub fn router_libraries(snapshot: &NetworkSnapshot) -> Vec<(String, Library)> {
    snapshot
        .nodes
        .iter()
        .filter_map(|node| {
            let LanNodeKind::Router(router) = &node.node else {
                return None;
            };

            library(&router.kernel_content).map(|library| (node.local_ip.to_string(), library))
        })
        .collect()
}

/// Finds a weakness and its memory address in `library`.
pub fn weakness(
    library: &Library,
    choose: impl Fn(&Weakness) -> bool,
) -> Option<(&str, &Weakness)> {
    library.zones.iter().find_map(|zone| {
        zone.weaknesses
            .iter()
            .find(|weakness| choose(weakness))
            .map(|weakness| (zone.address.as_str(), weakness))
    })
}

/// Whether `weakness` gives a shell without requirements.
pub fn is_free_shell(weakness: &Weakness) -> bool {
    weakness.reward == Reward::Shell && weakness.requirements.is_empty()
}

/// A script overflowing the selected weakness and printing its result's type. With `target`, the library comes from
/// `net_use` at that address and port; otherwise it is loaded from `path` on the player.
pub fn overflow_script(
    path: &str,
    target: Option<(&str, u16)>,
    address: &str,
    weakness: &Weakness,
    args: &str,
) -> String {
    let library = match target {
        Some((ip, port)) => format!("mx.net_use(\"{ip}\", {port}).dump_lib"),
        None => format!("mx.load(\"{path}\")"),
    };

    format!(
        "mx = include_lib(\"/lib/metaxploit.so\"); lib = {library}; result = lib.overflow(\"{address}\", \"{}\", \"{args}\"); print typeof(result)",
        weakness.unsec_value
    )
}
