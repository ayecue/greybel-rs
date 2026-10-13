//! Runtime protocol bindings for providers.

mod failure;
mod location;
mod step_mode;

duplex_core::include_protocol!("$PROTOCOLS/runtime", major = 1, role = provider);
