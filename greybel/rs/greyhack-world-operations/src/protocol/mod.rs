//! World-protocol bindings for the provider role.

duplex_core::include_protocol!("$PROTOCOLS/world", major = 1, role = provider);

mod common;
mod computer;
mod economy;
mod exploits;
mod files;
mod guilds;
mod intrusion;
mod network;
mod npcs;
mod player;
mod services;
mod shell;
mod smart_devices;
