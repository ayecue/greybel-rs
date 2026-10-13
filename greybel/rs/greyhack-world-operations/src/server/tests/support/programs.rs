use greyhack_world::ComputerId;

use crate::protocol::{ObjectId, ProgramId};
use crate::server::programs::{ProgramTable, TerminalInfo};

/// A session object.
pub(super) const SESSION: ObjectId = ObjectId { id: 0 };

/// A computer for the programs' processes. The program-table tests only need a stable id, not a real computer.
pub(super) fn computer() -> ComputerId {
    ComputerId::new(1)
}

/// Another computer, which a terminal can run on.
pub(super) fn remote() -> ComputerId {
    ComputerId::new(2)
}

/// A third computer, which a nested terminal can run on.
pub(super) fn far() -> ComputerId {
    ComputerId::new(3)
}

/// Starts a terminal on `host` for a program of `parent`'s tree, as `start_terminal` does.
pub(super) fn terminal_on(
    table: &mut ProgramTable,
    parent: ProgramId,
    host: greyhack_world::ComputerId,
    bounces: u32,
    launcher_bounces: u32,
) -> ProgramId {
    table.add_remote_terminal(0, parent, host);

    let program = table
        .add_program(0, None, SESSION, Some(parent))
        .expect("an id");

    table.set_terminal(
        program,
        TerminalInfo {
            host,
            bounces,
            launcher_bounces,
        },
    );

    program
}
