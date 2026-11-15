//! Deterministic generation of networks, computers, file systems, random content, and repositories.

pub(crate) mod computer;
mod filesystem;
pub mod lan;
pub(crate) mod network;
pub(crate) mod random_content;
mod repository;

pub use computer::{assign_player_libraries, router_computer};
pub use filesystem::{add_player_libraries, build_filesystem};
use greyhack_behaviour::{LibraryKind, NetworkKind, ROOT_USER, SYSTEM_PROCESSES};
use greyhack_world_core::{MinerHardware, Process};
use greyhack_world_libraries::{Library, LibraryGeneration as _};
pub use lan::{ComputerPlan, DevicePlan, LanPlan, ServicePlan};
pub use network::WifiGenerator;
pub use repository::RepositoryDraft;

use crate::Rng;

/// Returns `1.<minor>.0` with minor 0–2 so some libraries exceed base version `1.0.0`.
pub(crate) fn library_version(rng: &mut Rng) -> String {
    let minor = rng.range(0, 3);

    format!("1.{minor}.0")
}

/// Generates router `kernel_router.so` and its weaknesses.
#[must_use]
pub fn kernel_router(rng: &mut Rng) -> Box<Library> {
    let version = library_version(rng);

    Box::new(Library::generate(rng, LibraryKind::KernelRouter, &version))
}

/// Processes started on every computer at boot.
#[must_use]
pub fn system_processes() -> Vec<Process> {
    SYSTEM_PROCESSES
        .into_iter()
        .map(|(pid, name, ram_mb)| Process {
            pid,
            user: ROOT_USER.to_owned(),
            name: name.to_owned(),
            cpu: 0.0,
            ram_mb,
            protected: true,
            service: None,
            is_terminal: false,
        })
        .collect()
}

/// Optional network overrides. Missing values are drawn from world random state.
#[derive(Debug, Clone, Default)]
pub struct NetworkOptions {
    /// Public address of the main router.
    pub public_ip: Option<String>,
    /// Wireless network name.
    pub essid: Option<String>,
    /// Wireless password.
    pub wifi_password: Option<String>,
    /// Domain name. Generates an unused domain if omitted.
    pub domain: Option<String>,
    /// Map coordinates. Derived from the public IP if omitted.
    pub position: Option<[f32; 2]>,
    /// Initial computer count.
    pub computers: Option<u32>,
}

/// Optional computer overrides. Missing values are drawn from world random state.
#[derive(Debug, Clone, Default)]
pub struct ComputerOptions {
    /// Computer name.
    pub name: Option<String>,
    /// Requested LAN address.
    pub lan_ip: Option<String>,
    /// Root account password.
    pub root_password: Option<String>,
    /// Non-root usernames and passwords.
    pub users: Option<Vec<(String, String)>>,
    /// Service names from [`COMPUTER_SERVICES`](greyhack_behaviour::COMPUTER_SERVICES).
    pub services: Option<Vec<String>>,
    /// Whether the device is a switch.
    pub is_switch: bool,
    /// Mining hardware installed on the computer.
    pub miner_hardware: MinerHardware,
}

impl ComputerOptions {
    /// Options for the first network computer, whose services determine network `kind`.
    #[must_use]
    pub fn primary(kind: NetworkKind) -> Self {
        Self {
            services: Some(
                greyhack_behaviour::primary_services(kind)
                    .iter()
                    .map(|service| (*service).to_owned())
                    .collect(),
            ),
            ..Self::default()
        }
    }

    /// Options for a computer of a network of `kind`: the primary options for the `primary`
    /// computer, the defaults for the others.
    #[must_use]
    pub fn for_computer(kind: NetworkKind, primary: bool) -> Self {
        if primary {
            Self::primary(kind)
        } else {
            Self::default()
        }
    }
}
