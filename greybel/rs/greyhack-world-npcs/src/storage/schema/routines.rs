//! Tables of NPC routine kinds.

use greyhack_world_storage::TableSet;

/// The data of NPC-switch routines.
pub(crate) const NPC_SWITCHES: TableSet = TableSet::new(
    "CREATE TABLE IF NOT EXISTS routine_npc_switches (
    routine_id INTEGER PRIMARY KEY REFERENCES routines(id) ON DELETE CASCADE,
    npc_id     INTEGER NOT NULL UNIQUE REFERENCES npcs(person_id) ON DELETE CASCADE
);

",
    &["routine_npc_switches"],
);
