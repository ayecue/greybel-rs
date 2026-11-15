//! Tables and open-time cleanup owned by the traces domain.

mod traces;

pub(crate) use traces::GROUP;
pub use traces::{ACTIVE_TRACES, PASSIVE_TRACES};

/// Deletes active traces, whose program lifetime does not survive reopening the world.
pub(crate) const VOLATILE: greyhack_world_storage::TableSet =
    greyhack_world_storage::TableSet::volatile("DELETE FROM active_traces;\n");
