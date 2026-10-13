//! Cross-entity behavior implemented on [`crate::World`], one folder per domain:
//!
//! - `computer`: hardware, startup, logs, accounts, processes, network cards, and files.
//! - `creation`: seed-driven creation of networks and computers.
//! - `economy`: coins, wallets, bank accounts, and the market.
//! - `exploits`: attacks against libraries, the hack shop, Metaxploit, and debug libraries.
//! - `guilds`: guilds, companies, CrownShield, and traces.
//! - `intrusion`: sniffers, Wi-Fi captures, encryption, and floods.
//! - `network`: topology, reachability, LANs, routers, and domains.
//! - `npcs`: NPCs, jobs, and mail.
//! - `player`: what the player's scripts look up: routers, folders, CTF events, and missions.
//! - `services`: service installation, control, and package repositories.
//! - `shell`: what a script's shell builds, connects to, launches, and ends.
//! - `smart_devices`: smart appliances, cameras, and police traffic cameras.

pub(crate) mod computer;
pub(crate) mod creation;
pub(crate) mod economy;
pub(crate) mod exploits;
pub(crate) mod guilds;
pub(crate) mod intrusion;
pub(crate) mod network;
pub(crate) mod npcs;
pub(crate) mod player;
pub(crate) mod services;
pub(crate) mod shell;
pub(crate) mod smart_devices;
