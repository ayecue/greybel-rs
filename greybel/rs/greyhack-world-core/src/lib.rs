#![deny(clippy::await_holding_refcell_ref)]

//! Core of the Grey Hack world: the [`World`] and its lifecycle, computers and networks with their
//! tables, and the service contracts through which they reach the domains above them.
//!
//! Base behaviour is exposed through [`CoreModule::computers`], [`CoreModule::files`], and
//! [`CoreModule::networks`], with core-owned routes. Models, routes, storage, systems, and world
//! lifecycle each live in their own module; higher domains add state and storage around the base
//! tables.

pub mod error;
pub mod model;
pub mod objects;
pub mod routes;
pub mod storage;
pub mod systems;
pub mod world;

pub use error::{LayoutFailure, Result, WorldError};
pub use model::{
    CoinId, Computer, ComputerDraft, ComputerId, ConnectionFailure, Contact, CopyReport, Cpu,
    DraftKind, FileFailure, FileSystemDraft, Gpu, HardDisk, Hardware, LanPlace, Library, LibraryId,
    LoginFailure, MinerHardware, Motherboard, NetCard, NetCardFailure, Network, NetworkDraft,
    NetworkId, Node, NodeDraft, NodeId, NodeKind, Part, Person, PersonId, Port, PortForward,
    PowerSupply, Process, ProcessFailure, Ram, Router, RouterDraft, RouterId, SavedNetwork,
    SavedWifiFailure, ServiceFailure, StartupFailure, TraceKind, User, UserFailure,
};
pub use objects::{
    Access, AccessHandle, AccessKind, FileHandle, FileRef, ObjectTableExt, ReverseShellHandle,
    RouterHandle,
};
pub use storage::Database;
pub use storage::schema::{
    BASE_TABLES, CORE_TABLES, PASSWORD_TABLES, PERSON_TABLES, WORLD_META_TABLES,
};
pub use systems::computer::failure::ComputerFailure;
pub use systems::computer::filesystem::script::failure::ScriptFileFailure;
pub use systems::computer::filesystem::script::transfer::{
    DeletionOrigin, ScriptCopy, TraceAction,
};
pub use systems::computer::library_version_service::LibraryVersionService;
pub use systems::computer::logs::LogWrite;
pub use systems::computer::net_cards::script::{
    ActiveCard, NetworkDevice, NetworkSwitch, SwitchResult, WifiRequest, WifiScan,
};
pub use systems::computer::player_trace_service::PlayerTraceService;
pub use systems::computer::processes::scripts::{ProcessClose, ProcessUsage};
pub use systems::computer::users::accounts::Login;
pub use systems::computer::users::network_admin_lookup::NetworkAdminLookup;
pub use systems::computer::users::npc_state::{NpcLookup, NpcState, NpcStatus};
pub use systems::network::connect::{LoginPacket, ServiceHandshake};
pub use systems::network::generation::NetworkGeneration;
pub use systems::network::lan::TablePort;
pub use systems::network::routers::PortLookup;
pub use systems::{ComputerService, CoreModule, FileSystemService, NetworkService};
pub use world::{ClockAdvance, ClockAdvanceEnd, PersistentState, RebootEvent, World, WorldRef};
