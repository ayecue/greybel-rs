//! Seeded world generation. Contains options, the top-level [`world`] builder, and subsystem
//! generators.

mod blockchain;
pub(crate) mod challenge;
pub(crate) mod computer;
mod filesystem;
pub(crate) mod lan;
mod library;
pub(crate) mod missions;
pub(crate) mod network;
pub(crate) mod npc_mail;
mod player_world;
mod preset;
pub(crate) mod random_content;
mod repository;
pub(crate) mod words;
mod zero_day;

use greyhack_behaviour::{LibraryKind, NetworkKind};
pub(crate) use player_world::player_world;
pub use preset::{PlayerOptions, system_processes};

use crate::{Library, MiningHardware, Rng};

/// Returns `1.<minor>.0` with minor 0–2 so some libraries exceed base version `1.0.0`.
pub(crate) fn library_version(rng: &mut Rng) -> String {
    let minor = rng.range(0, 3);

    format!("1.{minor}.0")
}

/// Generates router `kernel_router.so` and its weaknesses.
pub(crate) fn kernel_router(rng: &mut Rng) -> Box<Library> {
    let version = library_version(rng);

    Box::new(Library::generate(rng, LibraryKind::KernelRouter, &version))
}

/// Configuration for world generation.
#[derive(Debug, Clone)]
pub struct WorldOptions {
    pub seed: u64,
    /// Additional network count beyond the player network.
    pub networks: u32,
    pub player: PlayerOptions,
}

impl Default for WorldOptions {
    fn default() -> Self {
        Self {
            seed: 0,
            networks: 5,
            player: PlayerOptions::default(),
        }
    }
}

/// Optional network overrides. Missing values are drawn from world random state.
#[derive(Debug, Clone, Default)]
pub struct NetworkOptions {
    pub public_ip: Option<String>,
    pub essid: Option<String>,
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
    pub name: Option<String>,
    pub lan_ip: Option<String>,
    pub root_password: Option<String>,
    /// Non-root usernames and passwords.
    pub users: Option<Vec<(String, String)>>,
    /// Service names from [`COMPUTER_SERVICES`](greyhack_behaviour::COMPUTER_SERVICES).
    pub services: Option<Vec<String>>,
    /// Whether the device is a switch.
    pub is_switch: bool,
    /// Mining hardware installed on the computer.
    pub mining_hardware: MiningHardware,
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
