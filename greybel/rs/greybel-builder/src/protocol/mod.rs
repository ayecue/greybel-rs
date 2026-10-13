//! Builder protocol bindings for providers.

mod failure;
mod location;
mod transpile;

duplex_core::include_protocol!("$PROTOCOLS/builder", major = 1, role = provider);
