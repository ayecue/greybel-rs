//! Tables of service-domain routine kinds.

use greyhack_world_storage::TableSet;

/// The data of service-restart routines.
pub(crate) const SERVICE_RESTARTS: TableSet = TableSet::new(
    "CREATE TABLE IF NOT EXISTS routine_service_restarts (
    routine_id  INTEGER PRIMARY KEY REFERENCES routines(id) ON DELETE CASCADE,
    computer_id INTEGER NOT NULL REFERENCES computers(id) ON DELETE CASCADE,
    service     INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS routine_service_restarts_computer ON routine_service_restarts (computer_id);

",
    &["routine_service_restarts"],
);
