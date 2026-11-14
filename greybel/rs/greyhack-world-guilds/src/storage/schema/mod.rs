//! Tables and open-time cleanup owned by the guilds domain.

mod guilds;
mod traces;

pub(crate) use guilds::GROUP;
pub(crate) use traces::GROUP as TRACE_GROUP;
pub use traces::{ACTIVE_TRACES, PASSIVE_TRACES};

/// Deletes active traces, whose program lifetime does not survive reopening the world.
pub(crate) const VOLATILE: greyhack_world_storage::TableSet =
    greyhack_world_storage::TableSet::volatile("DELETE FROM active_traces;\n");
