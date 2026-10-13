//! Reads and updates of per-program state for operations.

use greyhack_world::ComputerId;

use super::ServerState;
use super::programs::Process;
use crate::protocol::{ObjectId, ProgramId, ScriptFailure};
use crate::{OperationError, Result};

impl ServerState {
    /// Returns the session of `connection`'s running `program`.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptFailure::ProgramEnded`] for an ended program or wrong connection.
    pub(crate) fn session(&self, connection: u32, program: ProgramId) -> Result<ObjectId> {
        self.programs
            .borrow()
            .session(connection, program)
            .ok_or(OperationError::Script(ScriptFailure::ProgramEnded))
    }

    /// Makes shell `session` the session of `connection`'s `program`, as required by `sudo`.
    pub(crate) fn set_session(&self, connection: u32, program: ProgramId, session: ObjectId) {
        self.programs
            .borrow_mut()
            .set_session(connection, program, session);
    }

    /// Adds `computer` to the terminal list for the tree of `connection`'s `program`.
    pub(crate) fn add_terminal_computer(
        &self,
        connection: u32,
        program: ProgramId,
        computer: ComputerId,
    ) {
        self.programs
            .borrow_mut()
            .add_terminal_computer(connection, program, computer);
    }

    /// Adds `computer` to the terminals the tree of `connection`'s `program` has started.
    pub(crate) fn add_remote_terminal(
        &self,
        connection: u32,
        program: ProgramId,
        computer: ComputerId,
    ) {
        self.programs
            .borrow_mut()
            .add_remote_terminal(connection, program, computer);
    }

    /// Returns terminal computers for the tree of `connection`'s `program`.
    pub(crate) fn terminal_computers(
        &self,
        connection: u32,
        program: ProgramId,
    ) -> Vec<ComputerId> {
        self.programs
            .borrow()
            .terminal_computers(connection, program)
    }

    /// Returns the root process for the tree of `connection`'s `program`.
    pub(crate) fn root_process(&self, connection: u32, program: ProgramId) -> Option<Process> {
        self.programs.borrow().root_process(connection, program)
    }

    /// Removes the process of `connection`'s launched `program`, closed by `start_terminal`.
    pub(crate) fn take_launched_process(
        &self,
        connection: u32,
        program: ProgramId,
    ) -> Option<Process> {
        self.programs
            .borrow_mut()
            .take_launched_process(connection, program)
    }

    /// Returns the number of launches between `program` and its runtime-started root.
    pub(crate) fn depth(&self, program: ProgramId) -> u32 {
        self.programs.borrow().depth(program)
    }

    /// Returns the process of the command a terminal runs, when `connection`'s `program` is one.
    pub(crate) fn terminal_command_process(
        &self,
        connection: u32,
        program: ProgramId,
    ) -> Option<Process> {
        self.programs
            .borrow()
            .terminal_command_process(connection, program)
    }
}
