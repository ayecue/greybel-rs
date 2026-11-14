//! Library weaknesses, zones, and reverse shells.

pub(crate) mod reverse_shell;
pub(crate) mod weakness;

pub use reverse_shell::ReverseShell;
pub use weakness::{Library, MemoryZone, Weakness};
