//! Helpers of the NPC mail parity tests.

use greyhack_test_support_client::TestClient;
use greyhack_test_support_client::protocol::test_support::{NetworkSnapshot, NpcSnapshot};

use super::game::{join_network, join_network_of_kind_where};

/// The wait between mail methods, longer than their two-second cooldown.
pub const COOLDOWN: &str = "wait(2.1)";

/// NPCs with a mail account, in the game's order.
pub fn mail_npcs(snapshot: &NetworkSnapshot) -> Vec<&NpcSnapshot> {
    snapshot
        .npcs
        .iter()
        .filter(|npc| npc.mail.is_some())
        .collect()
}

/// The GreyScript list of `[address, password]` pairs of `npcs`.
pub fn accounts(npcs: &[&NpcSnapshot]) -> String {
    npcs.iter()
        .filter_map(|npc| npc.mail.as_ref())
        .map(|mail| format!(r#"["{}", "{}"]"#, mail.address, mail.password))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The GreyScript list of the addresses of `npcs`.
pub fn addresses(npcs: &[&NpcSnapshot]) -> String {
    npcs.iter()
        .filter_map(|npc| npc.mail.as_ref())
        .map(|mail| format!(r#""{}""#, mail.address))
        .collect::<Vec<_>>()
        .join(", ")
}

/// An NPC that can be dismissed: not the administrator, whose account the game refreshes another way, and not the
/// last NPC with a mail account, to whom the test sends a mail.
pub fn dismissal(snapshot: &NetworkSnapshot) -> Option<&NpcSnapshot> {
    let npcs = mail_npcs(snapshot);
    let admin = snapshot.admin.as_ref().map(|admin| admin.user.to_string());

    npcs[..npcs.len().saturating_sub(1)]
        .iter()
        .find(|npc| admin.as_deref() != Some(&*npc.user))
        .copied()
}

/// A mail network with at least four NPCs that have mail accounts, one of which `dismissal` finds. It is a network
/// the copy of the game has not generated before: the fixed network every test starts on keeps what earlier tests did
/// to it, and a library update an administrator reported there makes the game's next read of a LAN router of it
/// write inside its own open read of that router's row (`Database.RestoreLanRouter`,
/// `Database.ApplyReportedLibUpdates`, `Database.SyncFileSystemDB`), which SQLite refuses as "database is locked"
/// after its 30-second wait.
pub async fn join_mail_network(client: &TestClient) -> NetworkSnapshot {
    let network = client
        .find_new_network("MailServices", "smtp")
        .await
        .expect("a network is generated");

    join_network(client, &network.public_ip).await;

    join_network_of_kind_where(client, &["MailServices"], "smtp", |snapshot| {
        snapshot.kind == "MailServices"
            && !snapshot.disabled
            && mail_npcs(snapshot).len() >= 4
            && dismissal(snapshot).is_some()
    })
    .await
}
