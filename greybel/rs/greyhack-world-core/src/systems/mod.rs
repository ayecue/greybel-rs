//! Core services: computers and their files, and networks.

pub mod computer;
mod module;
pub mod network;

pub use computer::ComputerService;
pub use computer::filesystem::FileSystems;
pub use computer::filesystem::changes::FileChanges;
pub use computer::filesystem::libraries::LibraryFiles;
pub use computer::filesystem::lookup::FileLookup;
pub use computer::filesystem::permissions::FilePermissions;
pub use computer::filesystem::script::ScriptFiles;
pub use computer::filesystem::sizes::FileSizes;
pub use computer::hardware::Hardware;
pub use computer::logs::Logs;
pub use computer::net_cards::NetCardService;
pub use computer::processes::ProcessService;
pub use computer::startup::Startup;
pub use computer::users::UserService;
pub use module::{CoreContext, CoreModule};
pub use network::NetworkService;
pub use network::connect::NetworkConnect;
pub use network::dns::Dns;
pub use network::lan::NetworkLan;
pub use network::reach::NetworkReach;
pub use network::router_config::RouterConfig;
pub use network::routers::RouterService;
pub use network::settings::NetworkSettings;
pub use network::topology::NetworkTopology;
