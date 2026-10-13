//! Rows deleted when a database opens.

/// Deletes program-lifetime rows: script processes, script logins, mining, sniffer and flood
/// subscribers, and mining rounds. Retains system processes, service daemons, reverse-shell
/// clients, and NPC logins.
pub(crate) const VOLATILE: &str = "
DELETE FROM mining_subscribers;
DELETE FROM sniffer_subscribers;
DELETE FROM flood_subscribers;
DELETE FROM active_traces;
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
";
