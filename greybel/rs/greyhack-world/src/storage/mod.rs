//! Persistence of the domains above core: schemas and one repository per table area.

pub(crate) mod open;
pub(crate) mod routines;
pub(crate) mod schema;

pub use greyhack_world_core::Database;
pub use open::DatabaseOpen;
pub(crate) use routines::RoutineRepository;
