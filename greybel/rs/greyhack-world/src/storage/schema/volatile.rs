//! Rows deleted when a database opens outside the domain-owned volatile sets.

use greyhack_world_storage::TableSet;

/// Deletes program-lifetime rows: script processes, script logins, mining, mining rounds, and the
/// uses and reports of library weaknesses. Retains system processes, service daemons, reverse-shell
/// clients, and NPC logins.
pub(super) const VOLATILE: TableSet = TableSet::volatile(
    "
DELETE FROM mining_subscribers;
DELETE FROM library_weakness_uses;
DELETE FROM library_reporters;
DELETE FROM computer_processes
WHERE service IS NULL
  AND protected = 0
  AND NOT EXISTS (
      SELECT 1 FROM reverse_shells
      WHERE reverse_shells.computer_id = computer_processes.computer_id AND reverse_shells.pid = computer_processes.pid
  );
DELETE FROM computer_sessions
WHERE NOT EXISTS (
    SELECT 1 FROM npcs JOIN computer_users ON computer_users.id = npcs.user_id
    WHERE npcs.online = 1
      AND computer_users.computer_id = computer_sessions.computer_id
      AND computer_users.name = computer_sessions.user
);
DELETE FROM routines WHERE id IN (SELECT routine_id FROM routine_mining_rounds);
",
);
