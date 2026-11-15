#![deny(clippy::await_holding_refcell_ref)]

//! Core of the Grey Hack world: the [`World`] and its lifecycle, computers and networks with their
//! tables, and the service contracts through which they reach the domains above them.
//!
//! Base behaviour is exposed through [`CoreModule::computers`], [`CoreModule::files`], and
//! [`CoreModule::networks`], with core-owned routes. Higher domains add their state and storage around
//! the base tables.

mod account_files;
mod call_future;
mod domain;
pub mod error;
mod events;
mod library_version;
pub mod model;
mod network_admins;
mod network_generation;
mod npc_state;
pub mod objects;
mod persistent_state;
mod player_trace;
pub mod routes;
pub mod storage;
pub mod systems;
pub mod world;

pub(crate) use account_files::AccountFileService;
pub use call_future::CallFuture;
pub use domain::CoreDomain;
pub use error::{LayoutFailure, Result, WorldError};
pub use events::{LoginPacket, TraceAction};
pub use greyhack_behaviour::Rng;
pub use greyhack_world_modules::{Dep, WorkId};
pub use library_version::LibraryVersionService;
pub use model::{
    CoinId, Computer, ComputerDraft, ComputerId, ConnectionFailure, Contact, CopyReport, Cpu,
    DraftKind, FileFailure, FileSystemDraft, Gender, Gpu, HardDisk, Hardware, LanFailure, LanPlace,
    Library, LibraryId, LoginFailure, MinerHardware, Motherboard, NetCard, NetCardFailure, Network,
    NetworkDraft, NetworkId, Node, NodeDraft, NodeId, NodeKind, Part, Person, PersonId, Port,
    PortForward, PowerSupply, Process, ProcessFailure, Ram, Router, RouterDraft, RouterId,
    SavedNetwork, SavedWifiFailure, ServiceFailure, StartupFailure, TraceKind, User, UserFailure,
};
pub use network_admins::NetworkAdminLookup;
pub use network_generation::NetworkGeneration;
pub use npc_state::{NpcLookup, NpcState, NpcStatus};
pub use objects::{
    Access, AccessHandle, AccessKind, FileHandle, FileRef, ObjectTableExt, ReverseShellHandle,
    RouterHandle,
};
pub use persistent_state::PersistentState;
pub use player_trace::PlayerTraceService;
pub use storage::Database;
pub use storage::schema::{
    BASE_TABLES, CORE_TABLES, PASSWORD_TABLES, PERSON_TABLES, WORLD_META_TABLES,
};
pub use systems::computer::failure::ComputerFailure;
pub use systems::computer::filesystem::script::failure::ScriptFileFailure;
pub use systems::computer::filesystem::script::transfer::{DeletionOrigin, ScriptCopy};
pub use systems::computer::logs::LogWrite;
pub use systems::computer::net_cards::script::{
    ActiveCard, NetworkDevice, NetworkSwitch, SwitchResult, WifiRequest, WifiScan,
};
pub use systems::computer::processes::scripts::{ProcessClose, ProcessUsage};
pub use systems::computer::users::accounts::Login;
pub use systems::network::connect::ServiceHandshake;
pub use systems::network::lan::TablePort;
pub use systems::network::routers::PortLookup;
pub use systems::{ComputerService, CoreModule, FileSystemService, NetworkService};
pub use world::{ClockAdvance, ClockAdvanceEnd, RebootEvent, World as WorldState, World, WorldRef};
