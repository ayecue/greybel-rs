//! Generic database layer of the Grey Hack world: no game rules.
//!
//! [`Database`] owns the connection and its per-cycle transaction. [`sql`] has the statement and
//! row-reading helpers. [`Schema`] collects the [`TableSet`]s that domains register, and a
//! database opened with it has exactly those tables.
//!
//! # Dependency order
//!
//! A schema orders domain tables by their declared dependencies, breaking ties by registration
//! order, and [`Schema::tables`] lists tables in that order. A database copy inserts tables in that
//! order and deletes them in reverse.

mod database;
mod domain_schema;
mod schema;
pub mod sql;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod web_io;

pub use database::Database;
pub use domain_schema::{DomainSchema, Migration, MigrationFuture, MigrationStep, SCHEMA_VERSIONS};
pub use schema::{Schema, TableSet};
