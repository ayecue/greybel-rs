#![deny(clippy::await_holding_refcell_ref)]

//! Deterministic seed-generated Grey Hack world with SQLite persistence and scheduling. Independent
//! of the interpreter.
//!
//! [`assemble`] builds the core and domain modules. World generation remains here; each domain
//! crate owns its services, entities, schema, routines, and routes.

// World futures are single-threaded (the world is not `Send`), so the futures of the extension
// traits need no `Send` bound.
#![allow(async_fn_in_trait)]

pub(crate) mod generation;
pub mod modules;
pub mod routes;
pub(crate) mod routines;
pub(crate) mod storage;
mod world;

pub use generation::{PlayerOptions, WorldOptions};
pub use modules::{ModuleSet, WorldAssembly, assemble};
pub use routines::{RoutineReport, RoutineService, ScheduledRoutine, TickReport};
pub use storage::DatabaseOpen;
pub use world::World;
