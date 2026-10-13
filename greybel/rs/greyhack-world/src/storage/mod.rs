//! World persistence: schema, connection, and table-group queries implemented as `impl Database`
//! blocks.

mod computers;
mod ctf;
mod database;
mod debug;
mod economy;
mod exploits;
mod guilds;
mod hardware;
mod libraries;
mod mail;
pub(crate) mod nodes;
mod npcs;
mod repositories;
mod routers;
mod routines;
pub(crate) mod schema;
pub(crate) mod sql;
mod subscribers;
mod traces;
mod users;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod web_io;
pub(crate) mod world_meta;

pub use database::Database;
