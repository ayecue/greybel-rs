//! Network types, deterministic address mappings, and identifying services.

use crate::{ServiceId, WorldSeed};

/// Filename of the router kernel library in `/lib`.
pub const KERNEL_ROUTER: &str = "kernel_router.so";
/// Maximum computers per network (`192.168.x.2` through `.254`).
pub const MAX_NETWORK_COMPUTERS: u32 = 253;
/// Available connection speeds for ISP packs S through XXL (`ISPPanel.PackID`).
pub const NET_SPEEDS: [u8; 5] = [3, 6, 12, 20, 25];
/// Default M-pack connection speed (`NetworkLan.netSpeed`).
pub const DEFAULT_NET_SPEED: u8 = 6;

/// Returns the number of concurrent flood sources that reboot a computer
/// (`NetSessionIntrinsics.flood_connection`): `net_speed` × 3, and 25% fewer without a super
/// administrator on the network.
#[must_use]
pub fn flood_reboot_threshold(net_speed: u8, super_admin: bool) -> u32 {
    let threshold = u32::from(net_speed) * 3;

    if super_admin {
        threshold
    } else {
        (threshold * 3).div_ceil(4)
    }
}

/// World-seed label for the public-address/network-type permutation (`IpGenerator`).
const IP_KIND_LABEL: &str = "ip-type-v1";

const IP_NETWORK_KINDS: [NetworkKind; 16] = [
    NetworkKind::Police,
    NetworkKind::University,
    NetworkKind::Supermarket,
    NetworkKind::FastFood,
    NetworkKind::Garage,
    NetworkKind::MobileShop,
    NetworkKind::Hospital,
    NetworkKind::Bank,
    NetworkKind::Residential,
    NetworkKind::MailService,
    NetworkKind::HackShop,
    NetworkKind::ComputerShop,
    NetworkKind::NetServices,
    NetworkKind::HardwareManufacturer,
    NetworkKind::Neurobox,
    NetworkKind::CurrencyCreation,
];

/// Derives the network type by reversing the world-seed permutation of `address` modulo 16
/// This matches `IpGenerator.GetNetworkType`.
#[must_use]
pub fn kind_for_ip(seed: &WorldSeed, address: &str) -> NetworkKind {
    let ip = address.parse::<std::net::Ipv4Addr>().map_or(0, u32::from);
    let index = seed.depermute_u32(IP_KIND_LABEL, ip)
        % u32::try_from(IP_NETWORK_KINDS.len()).expect("value fits the target integer type");

    IP_NETWORK_KINDS[index as usize]
}

/// Converts the last two address octets to map coordinates.
#[must_use]
pub fn position_for_ip(address: &str) -> [f32; 2] {
    let parts: Vec<f32> = address
        .split('.')
        .map(|part| part.parse::<f32>().unwrap_or(0.0) / 255.0)
        .collect();

    [
        parts.get(2).copied().unwrap_or(0.0),
        parts.get(3).copied().unwrap_or(0.0),
    ]
}

/// Computes Euclidean distance between map positions.
#[must_use]
pub fn map_distance(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

enum_def! {
    /// Network purpose (`ServerMap.TipoRed`).
    pub enum NetworkKind {
        Unknown = 0,
        Police = 1,
        University = 2,
        Supermarket = 3,
        FastFood = 4,
        Garage = 5,
        MobileShop = 6,
        Hospital = 7,
        Bank = 8,
        Residential = 9,
        MailService = 10,
        HackShop = 11,
        ComputerShop = 12,
        RentServer = 13,
        NetServices = 14,
        HardwareManufacturer = 15,
        Neurobox = 16,
        CurrencyCreation = 17,
        Ctf = 18,
    }
}

impl NetworkKind {
    /// Network types used when the game selects a person (`OS.GetListTipoRedComun`). Excludes Unknown,
    /// MailService, RentServer, ComputerShop, HackShop, Bank, Neurobox, and Ctf.
    pub const COMMON: [Self; 11] = [
        Self::Police,
        Self::University,
        Self::Supermarket,
        Self::FastFood,
        Self::Garage,
        Self::MobileShop,
        Self::Hospital,
        Self::Residential,
        Self::NetServices,
        Self::HardwareManufacturer,
        Self::CurrencyCreation,
    ];

    /// Whether the player makes networks of this kind, which have no administrator and no traces
    /// (`Computer.IsSpawnedByPlayer`, `Computer.GetAdmin`).
    #[must_use]
    pub const fn is_player_made(self) -> bool {
        matches!(self, Self::RentServer | Self::Ctf | Self::Unknown)
    }

    /// Returns the `ServerMap.TipoRed` table key for this network type.
    #[must_use]
    pub const fn game_name(self) -> &'static str {
        match self {
            Self::Unknown => "Unknown",
            Self::Police => "Comisaria",
            Self::University => "Universidades",
            Self::Supermarket => "Supermercados",
            Self::FastFood => "FastFood",
            Self::Garage => "Taller",
            Self::MobileShop => "MobileShop",
            Self::Hospital => "Hospitales",
            Self::Bank => "Bancos",
            Self::Residential => "Particulares",
            Self::MailService => "MailServices",
            Self::HackShop => "HackShop",
            Self::ComputerShop => "TiendaInformatica",
            Self::RentServer => "RentServer",
            Self::NetServices => "NetServices",
            Self::HardwareManufacturer => "HardwareManufacturer",
            Self::Neurobox => "Neurobox",
            Self::CurrencyCreation => "CurrencyCreation",
            Self::Ctf => "CTF",
        }
    }

    /// Looks up a network type by `name` from `ServerMap.TipoRed`.
    #[must_use]
    pub fn of_game_name(name: &str) -> Option<Self> {
        Some(match name {
            "Unknown" => Self::Unknown,
            "Comisaria" => Self::Police,
            "Universidades" => Self::University,
            "Supermercados" => Self::Supermarket,
            "FastFood" => Self::FastFood,
            "Taller" => Self::Garage,
            "MobileShop" => Self::MobileShop,
            "Hospitales" => Self::Hospital,
            "Bancos" => Self::Bank,
            "Particulares" => Self::Residential,
            "MailServices" => Self::MailService,
            "HackShop" => Self::HackShop,
            "TiendaInformatica" => Self::ComputerShop,
            "RentServer" => Self::RentServer,
            "NetServices" => Self::NetServices,
            "HardwareManufacturer" => Self::HardwareManufacturer,
            "Neurobox" => Self::Neurobox,
            "CurrencyCreation" => Self::CurrencyCreation,
            "CTF" => Self::Ctf,
            _ => return None,
        })
    }

    /// Returns public services in `TipoRedPublica` order, each with its percentage chance (100 means
    /// always) and whether its port starts closed.
    #[must_use]
    pub fn public_services(self) -> Vec<(ServiceId, u8, bool)> {
        greyhack_world_assets::public_services(self.game_name())
            .iter()
            .filter_map(|entry| {
                ServiceId::from_code(i64::from(entry.service))
                    .map(|service| (service, entry.chance, entry.closed))
            })
            .collect()
    }
}

/// Public services always installed on a generated network's first server. These supplement the full
/// weighted `TipoRedPublica` tables so each network type receives its identifying content.
#[must_use]
pub const fn primary_services(kind: NetworkKind) -> &'static [&'static str] {
    match kind {
        NetworkKind::Police => &["ssh", "ftp", "http", "criminals", "employees"],
        NetworkKind::University => &["http", "employees", "students", "ftp", "ssh"],
        NetworkKind::Bank => &["bank_account", "http"],
        NetworkKind::MailService => &["ssh", "ftp", "smtp", "http", "employees"],
        NetworkKind::HackShop => &["http", "repository"],
        NetworkKind::ComputerShop | NetworkKind::NetServices | NetworkKind::CurrencyCreation => {
            &["http", "employees"]
        }
        NetworkKind::Supermarket => &["employees", "ssh"],
        NetworkKind::FastFood => &["http", "ssh", "employees"],
        NetworkKind::Garage | NetworkKind::MobileShop => &["ftp", "ssh", "employees"],
        NetworkKind::Hospital => &["employees", "ssh", "ftp"],
        NetworkKind::HardwareManufacturer => &["http", "employees", "ssh", "ftp"],
        NetworkKind::Neurobox => &["employees"],
        NetworkKind::Residential => &["ssh", "ftp"],
        NetworkKind::RentServer => &["ssh"],
        NetworkKind::Unknown | NetworkKind::Ctf => &[],
    }
}
