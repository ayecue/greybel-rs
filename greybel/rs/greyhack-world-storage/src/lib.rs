//! Generic database layer of the Grey Hack world: no game rules.
//!
//! [`Database`] owns the connection and its per-cycle transaction. [`sql`] has the statement and
//! row-reading helpers. [`Schema`] collects the [`TableSet`]s that domains register, and a
//! database opened with it has exactly those tables.
//!
//! # Registration order
//!
//! A schema is created from its table sets in registration order, and
//! [`Schema::tables`] lists tables in the same order. Foreign keys therefore stay valid when a set
//! is registered after the sets it references: register parent tables first. A database copy
//! inserts tables in that order and deletes them in reverse.

mod database;
mod domain;
mod schema;
pub mod sql;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod web_io;

pub use database::Database;
pub use domain::{DomainSchema, Migration, MigrationFuture, MigrationStep, SCHEMA_VERSIONS};
pub use schema::{Schema, TableSet};
