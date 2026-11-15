//! Persistence of the domains above core: schemas and one repository per table area.

pub(crate) mod routines;
pub(crate) mod schema;
pub(crate) mod world_database;

pub(crate) use routines::RoutineRepository;
pub use world_database::DatabaseOpen;
