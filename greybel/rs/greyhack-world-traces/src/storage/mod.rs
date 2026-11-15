//! Trace repositories and the tables they own.

pub mod schema;
mod traces;

pub(crate) use traces::TraceRepository;
