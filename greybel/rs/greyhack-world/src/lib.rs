//! Deterministic seed-generated Grey Hack world with SQLite persistence and scheduling. Independent
//! of the interpreter.
//!
//! `model` has the data types, `systems` the behavior across entities (one folder per domain),
//! `generation` the seeded generators, `storage` the database tables, `routines` the scheduled work,
//! and `world` the [`World`] itself.

mod error;
mod generation;
mod model;
mod routines;
mod storage;
mod systems;
mod world;

pub use error::{LayoutFailure, Result, WorldError};
pub use generation::computer::router_computer;
pub use generation::missions::procedural_mission_json;
pub use generation::words::Word;
pub use generation::{
    ComputerOptions, NetworkOptions, PlayerOptions, WorldOptions, system_processes,
};
pub use greyhack_behaviour::Rng;
pub(crate) use model::add_home_folders;
pub use model::{
    ActiveTrace, AdminRestoreFailure, AlarmConfig, AptFailure, BankAccount, BankLayout,
    BankSubscription, BankTransaction, BankTransferFailure, CameraFeed, CameraNpc, Coin, CoinId,
    CoinLayout, CompanyAlign, CompanyLayout, Computer, ComputerDraft, ComputerId,
    ConnectionFailure, Contact, CopyReport, Cpu, Crime, CrownShieldFailure, CrownState, CtfAccount,
    CtfEvent, CtfFailure, CtfPlayer, DEFAULT_DECODE_PATH, DEFAULT_ENCODE_PATH, DebugAccount,
    DebugChallenge, DebugLibraryState, DeviceLayout, EconomyLayout, Email, Exploit, ExploitOffer,
    FacePart, FileFailure, FileSystemDraft, ForwardLayout, Gender, Gpu, Grade, Guild, GuildAbsence,
    GuildDraft, GuildFailure, GuildId, GuildLayout, GuildsLayout, HardDisk, Hardware, HttpConfig,
    JobRole, LanFailure, LanNodeLayout, LanPlace, Library, LibraryId, LocatedVehicle, LoginFailure,
    MailAccount, MarketFailure, MemoryZone, MineJobFailure, MiningHardware, Motherboard, NetCard,
    NetCardFailure, NetUseFailure, NetUseSession, Network, NetworkAdmin, NetworkDraft, NetworkId,
    NetworkLayout, Node, NodeDraft, NodeId, NodeKind, Npc, NpcEvent, NpcSchedule, Offense, Offer,
    OfferLayout, Package, PackageEntry, Part, PassiveState, Person, PersonId, Port, PortForward,
    PowerSupply, PriceRecord, Process, ProcessFailure, PurchaseFailure, Ram, Record,
    RecordDatabase, RecordKind, Repository, RepositoryConfig, ReverseShell, Router, RouterDraft,
    RouterId, RshellFailure, SavedNetwork, SavedWifiFailure, ServiceFailure, Side,
    SmartApplianceSettings, SmtpFailure, SnifferPacket, SoftwareInfo, SshdConfig, Stake,
    StartupFailure, SubWallet, SubWalletLayout, Subject, TraceId, TraceKind, Trade, TrafficNet,
    Transaction, TransferFailure, User, UserFailure, Vector, Wallet, WalletId, WalletLayout,
    Weakness, WifiCapture, WorkPerformance, ZeroDay, ZeroDayObject, ZeroDayObjectKind,
};
pub use routines::{Routine, RoutineFailure, ScheduledRoutine, TickReport};
pub use storage::Database;
pub use systems::computer::failure::ComputerFailure;
pub use systems::computer::filesystem::script::failure::ScriptFileFailure;
pub use systems::computer::filesystem::script::transfer::{DeletionOrigin, ScriptCopy};
pub use systems::computer::logs::LogWrite;
pub use systems::computer::net_cards::script::{
    ActiveCard, NetworkDevice, NetworkSwitch, SwitchResult, WifiRequest, WifiScan,
};
pub use systems::computer::processes::scripts::{ProcessClose, ProcessUsage};
pub use systems::computer::users::accounts::Login;
pub use systems::economy::coin_scripts::Connectivity;
pub use systems::economy::failure::EconomyFailure;
pub use systems::exploits::debug::{DebugFailure, DebugLibraryPatch, DebugPayload, PartialObject};
pub use systems::exploits::library_updates::LibraryReportFailure;
pub use systems::exploits::metaxploit::rshell::{
    RshellClient, RshellClientFailure, RshellServerFailure,
};
pub use systems::exploits::metaxploit::scan::{LibraryScan, ZoneScan};
pub use systems::exploits::report::{
    AttackResult, AttackReward, ExploitLog, ExploitOutcome, ExploitReport, UnmetRequirement,
};
pub use systems::exploits::shop::failure::ExploitShopFailure;
pub use systems::exploits::shop::purchase::ExploitPurchase;
pub use systems::intrusion::capture::AckCount;
pub use systems::intrusion::crypto::{CryptoFailure, WifiCaptureRun};
pub use systems::intrusion::ddos::{FloodAttempt, FloodWarning};
pub use systems::intrusion::flood::Flood;
pub use systems::intrusion::sniffer::encoder::EncoderFailure;
pub use systems::network::connect::ServiceHandshake;
pub use systems::network::lan::TablePort;
pub use systems::network::routers::PortLookup;
pub use systems::npcs::alarm::{AlarmFailure, PhoneAlarm};
pub use systems::npcs::jobs::{FireFailure, HireFailure};
pub use systems::npcs::mail::messages::{MailRefusal, SendMailFailure};
pub use systems::npcs::rental_admin::RentalAdminFailure;
pub use systems::player::lookups::{
    CtfPasswordFailure, FolderChange, FolderLookup, IncludedLibrary, PlayerAccount,
};
pub use systems::player::missions::{AcceptedMission, MissionFailure};
pub use systems::services::apt::updates::AptUpdateLine;
pub use systems::services::control::{ServiceControlFailure, ServiceStart};
pub use systems::services::install::records::RecordFailure;
pub use systems::shell::build::{BuildFailure, BuiltSource, ImportFailure};
pub use systems::shell::connect::{Encryption, EncryptionScript, ServiceOpen, ServiceRequest};
pub use systems::shell::launch::{
    Launch, LaunchFailure, LaunchedProgram, TerminalFailure, TerminalProgram,
};
pub use systems::smart_devices::appliance::ApplianceFailure;
pub use systems::smart_devices::cctv::CctvFailure;
pub use systems::smart_devices::traffic::TrafficFailure;
pub use world::{MineJob, WorkEnd, WorkResult, World, WorldEvent};
