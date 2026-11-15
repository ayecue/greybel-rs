//! Core services: computers and their files, and networks.

pub mod computer;
mod module;
pub mod network;

pub use computer::ComputerService;
pub use computer::filesystem::FileSystemService;
pub use computer::filesystem::changes::FileChangeService;
pub use computer::filesystem::libraries::LibraryFileService;
pub use computer::filesystem::lookup::FileLookup;
pub use computer::filesystem::permissions::FilePermissionService;
pub use computer::filesystem::script::ScriptFileService;
pub use computer::filesystem::sizes::FileSizeService;
pub use computer::hardware::Hardware;
pub use computer::logs::LogService;
pub use computer::net_cards::NetCardService;
pub use computer::processes::ProcessService;
pub use computer::startup::Startup;
pub use computer::users::UserService;
pub use module::CoreModule;
pub use network::NetworkService;
pub use network::connect::NetworkConnect;
pub use network::dns::Dns;
pub use network::lan::NetworkLan;
pub use network::reach::NetworkReach;
pub use network::router_config::RouterConfig;
pub use network::routers::RouterService;
pub use network::settings::NetworkSettings;
pub use network::topology::NetworkTopology;
