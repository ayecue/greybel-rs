#![deny(clippy::await_holding_refcell_ref)]

//! Generic world-module assembly, lazy dependencies, and module capabilities.

mod capability;
mod dep;
mod module;
mod registry;

pub use capability::CapabilityRegistry;
pub use dep::Dep;
pub use module::Module;
pub use registry::{Assembly, Registry};
