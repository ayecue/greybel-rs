//! Grey Hack-compatible file operations: symlink-aware paths, replacement, tree copies and moves,
//! sizes, and permissions. All operations access node rows.

pub(crate) mod changes;
pub(crate) mod copy;
pub(crate) mod delete;
pub(crate) mod libraries;
pub(crate) mod lookup;
pub(crate) mod permissions;
pub(crate) mod script;
pub(crate) mod sizes;
pub(crate) mod write;
