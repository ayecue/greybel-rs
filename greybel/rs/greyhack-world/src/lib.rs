#![deny(clippy::await_holding_refcell_ref)]

//! Deterministic seed-generated Grey Hack world with SQLite persistence and scheduling. Independent
//! of the interpreter.
//!
//! [`assemble`] builds the core and domain modules. World generation remains here;
//! each domain crate owns its services, entities, schema, routines, and routes. Existing domain
//! APIs are re-exported from this composition root.

// World futures are single-threaded (the world is not `Send`), so the futures of the extension
// traits need no `Send` bound.
#![allow(async_fn_in_trait)]

pub mod creation;
pub mod economy;
pub(crate) mod generation;
pub mod guilds;
pub mod intrusion;
pub mod libraries;
pub(crate) mod model;
pub mod modules;
pub mod npcs;
pub mod objects;
pub mod player;
pub mod prelude;
pub mod routes;
pub(crate) mod routines;
pub mod services;
pub mod shell;
pub mod smart_devices;
pub(crate) mod storage;
mod world;

pub use creation::{
    ComputerOptions, DeviceLayout, ForwardLayout, LanNodeLayout, NetworkLayout, NetworkOptions,
    WifiGenerator, router_computer, system_processes,
};
pub use economy::coin_scripts::Connectivity;
pub use economy::failure::EconomyFailure;
pub use economy::{
    Bank, CoinService, EconomyModule, Market, MineJob, MineJobFailure, WalletService,
};
pub use generation::{PlayerOptions, WorldOptions};
pub use greyhack_behaviour::{Rng, Word};
pub use greyhack_world_core::{
    Access, AccessHandle, AccessKind, ActiveCard, ClockAdvanceEnd, ComputerFailure, Database,
    DeletionOrigin, FileHandle, FileRef, LayoutFailure, LogWrite, Login, NetworkDevice,
    NetworkSwitch, ObjectTableExt as CoreObjectTableExt, PortLookup, ProcessClose, ProcessUsage,
    RebootEvent, Result, ReverseShellHandle, RouterHandle, ScriptCopy, ScriptFileFailure,
    ServiceHandshake, SwitchResult, TablePort, WifiRequest, WifiScan, WorldError, WorldRef,
    WorldState,
};
pub use greyhack_world_missions::mission_json;
pub use greyhack_world_modules::{Work, WorkCancel, WorkEnvelope, WorkId, WorkQueue};
pub use greyhack_world_npcs::{
    AlarmFailure, FireFailure, HireFailure, MailRefusal, PhoneAlarm, RentalAdminFailure,
    SendMailFailure,
};
pub use guilds::StrikeEvent;
pub use intrusion::{
    AckCount, CryptoFailure, EncoderFailure, Flood, FloodAttempt, FloodWarning, SnifferPacket,
    WifiCapture, WifiCaptureRun,
};
pub use libraries::{
    AttackResult, AttackReward, DebugFailure, DebugLibraryPatch, DebugPayload, ExploitLog,
    ExploitOutcome, ExploitPurchase, ExploitReport, ExploitShopFailure, LibraryReportFailure,
    LibraryScan, PartialObject, RshellClient, RshellClientFailure, RshellServerFailure,
    UnmetRequirement, WeaknessReportFailure, ZoneScan,
};
pub use model::{
    ActiveTrace, AdminRestoreFailure, AlarmConfig, AptFailure, BankAccount, BankLayout,
    BankSubscription, BankTransaction, BankTransferFailure, Coin, CoinId, CoinLayout, CompanyAlign,
    CompanyLayout, Computer, ComputerDraft, ComputerId, ConnectionFailure, Contact, CopyReport,
    Cpu, Crime, CrownRank, CrownShieldFailure, CrownState, CtfAccount, CtfEvent, CtfFailure,
    CtfPlayer, DEFAULT_DECODE_PATH, DEFAULT_ENCODE_PATH, DebugAccount, DebugChallenge,
    DebugLibraryState, EconomyLayout, Email, Exploit, ExploitOffer, FacePart, FileFailure,
    FileSystemDraft, Gender, Gpu, Grade, Guild, GuildAbsence, GuildDraft, GuildFailure, GuildId,
    GuildLayout, GuildsLayout, HardDisk, Hardware, HttpConfig, JobRole, LanFailure, LanPlace,
    Library, LibraryId, LoginFailure, MailAccount, MarketFailure, MemoryZone, MiningHardware,
    Motherboard, NetCard, NetCardFailure, NetUseFailure, NetUseSession, Network, NetworkAdmin,
    NetworkDraft, NetworkId, Node, NodeDraft, NodeId, NodeKind, Npc, NpcEvent, NpcSchedule,
    Offense, Offer, OfferLayout, Package, PackageEntry, Part, PassiveState, Person, PersonId, Port,
    PortForward, PowerSupply, PriceRecord, Process, ProcessFailure, PurchaseFailure, Ram, Record,
    RecordDatabase, RecordKind, Repository, RepositoryConfig, ReverseShell, Router, RouterDraft,
    RouterId, RshellFailure, SavedNetwork, SavedWifiFailure, ServiceFailure, Side,
    SmartApplianceSettings, SmtpFailure, SoftwareInfo, SshdConfig, Stake, StartupFailure,
    SubWallet, SubWalletLayout, Subject, TraceId, TraceKind, Trade, Transaction, TransferFailure,
    User, UserFailure, Vector, Wallet, WalletId, WalletLayout, Weakness, WorkPerformance, ZeroDay,
    ZeroDayObject, ZeroDayObjectKind,
};
pub use modules::{ModuleSet, WorldAssembly, assemble};
pub use player::GameOverEvent;
pub use player::lookups::{
    CtfPasswordFailure, FolderChange, FolderLookup, IncludedLibrary, PlayerAccount,
};
pub use player::missions::{AcceptedMission, MissionFailure};
pub use routines::{RoutineFailure, ScheduledRoutine, TickReport};
pub use services::{
    AptUpdateLine, RecordFailure, ServiceControlFailure, ServiceRestartFailure, ServiceStart,
};
pub use shell::build::{BuildFailure, BuiltSource, ImportFailure};
pub use shell::connect::{Encryption, EncryptionScript, ServiceOpen, ServiceRequest};
pub use shell::launch::{Launch, LaunchFailure, LaunchedProgram, TerminalFailure, TerminalProgram};
pub use smart_devices::{
    ApplianceFailure, CameraFeed, CameraNpc, CctvFailure, LocatedVehicle, TrafficFailure,
    TrafficNet, TrafficState,
};
pub use world::World;
