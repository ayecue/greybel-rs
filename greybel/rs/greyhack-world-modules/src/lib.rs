#![deny(clippy::await_holding_refcell_ref)]

//! Generic world-module assembly, lazy dependencies, ordered events, and typed waiting work.

mod capability;
mod dep;
mod events;
mod module;
mod registry;
mod work;

pub use capability::CapabilityRegistry;
pub use dep::Dep;
pub use events::{Event, EventError, EventFuture, EventQueue};
pub use module::Module;
pub use registry::{Assembly, Registry};
pub use work::{
    Work, WorkCancel, WorkCancelFuture, WorkCompletion, WorkEnvelope, WorkId, WorkOwner, WorkQueue,
};
