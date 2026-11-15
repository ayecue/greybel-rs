//! NPC, mail, and NPC-switch routine tables.

pub(crate) mod mail;
pub(crate) mod npcs;
pub(crate) mod routines;

/// Deletes script logins while retaining sessions belonging to online NPCs.
pub(crate) const VOLATILE: greyhack_world_storage::TableSet =
    greyhack_world_storage::TableSet::volatile(
        "DELETE FROM computer_sessions
WHERE NOT EXISTS (
    SELECT 1 FROM npcs JOIN computer_users ON computer_users.id = npcs.user_id
    WHERE npcs.online = 1
      AND computer_users.computer_id = computer_sessions.computer_id
      AND computer_users.name = computer_sessions.user
);
",
    );
