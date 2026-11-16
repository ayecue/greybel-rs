//! Address helpers for computer parity fixtures.

use greyhack_test_support_client::protocol::test_support::{LanNodeKind, NetworkSnapshot};

/// Main-router gateway and player address in a snapshot.
pub fn ethernet_addresses(snapshot: &NetworkSnapshot) -> (String, String) {
    let gateway = snapshot
        .nodes
        .iter()
        .find(|node| node.parent.is_none())
        .expect("the main router")
        .local_ip
        .clone();
    let player = snapshot
        .nodes
        .iter()
        .find_map(|node| match &node.node {
            LanNodeKind::Computer(computer) if computer.player => Some(node.local_ip.clone()),
            _ => None,
        })
        .expect("the player computer");

    (gateway, player)
}

/// Replaces the final IPv4 octet for a free address in the fixture LAN.
pub fn replace_last_octet(address: &str, octet: &str) -> String {
    let (prefix, _) = address.rsplit_once('.').expect("an IPv4 address");

    format!("{prefix}.{octet}")
}
