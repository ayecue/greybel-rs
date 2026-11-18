//! Apt snapshot helpers.

use greyhack_test_support_client::protocol::test_support::{
    FileSnapshot, LanNodeKind, NetworkSnapshot,
};

/// Returns the player's file at `path` in `snapshot`.
pub fn player_file<'a>(snapshot: &'a NetworkSnapshot, path: &str) -> Option<&'a FileSnapshot> {
    snapshot
        .nodes
        .iter()
        .find_map(|node| match &node.node {
            LanNodeKind::Computer(computer) if computer.player => Some(computer),
            _ => None,
        })
        .expect("the snapshot contains the player computer")
        .files
        .iter()
        .find(|file| file.path == path)
}
