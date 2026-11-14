//! Connection channels and server-wide program ID allocation.

use std::collections::HashMap;
use std::rc::Rc;

use greyhack_world_router::TreeId;

use super::ConnectionChannel;
use crate::protocol::ProgramId;

/// Connection channels and server-wide ID allocation.
#[derive(Default)]
pub(crate) struct TreeIndex {
    connections: HashMap<u32, ConnectionChannel>,
    next_connection: u32,
    next_program: u32,
}

impl TreeIndex {
    pub(crate) fn add_connection(&mut self, channel: ConnectionChannel) -> u32 {
        let id = self.next_connection;

        self.next_connection += 1;
        self.connections.insert(id, channel);

        id
    }

    /// Removes `connection`. The caller drops the channel after releasing the index borrow.
    pub(crate) fn remove_connection(&mut self, connection: u32) -> Option<ConnectionChannel> {
        self.connections.remove(&connection)
    }

    /// Allocates a program ID. Parent and tree association is maintained by the program-tree pool.
    pub(crate) fn add_program(&mut self, _connection: u32) -> Option<(ProgramId, TreeId)> {
        let id = self.next_program;

        self.next_program = self.next_program.checked_add(1)?;

        let program = ProgramId { id };

        Some((program, TreeId::new(id)))
    }

    /// Returns the notification channel of `key`'s connection.
    pub(crate) fn channel(&self, key: TreeId, connection: u32) -> Option<ConnectionChannel> {
        let _ = key;

        self.connections.get(&connection).map(Rc::clone)
    }
}
