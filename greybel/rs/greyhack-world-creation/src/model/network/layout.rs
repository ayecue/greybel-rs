use greyhack_behaviour::{LibraryKind, NetworkAdmin, NetworkKind};
use greyhack_world_core::{Contact, Port};
use greyhack_world_libraries::{ComputerDraft, RouterDraft};

/// Explicit node-by-node network layout used to reproduce another network, including layouts copied
/// from Grey Hack.
#[derive(Debug, Clone)]
pub struct NetworkLayout {
    pub kind: NetworkKind,
    pub public_ip: String,
    /// Main router position on the world map.
    pub position: [f32; 2],
    pub domain: String,
    /// Administrative contact returned by `whois`, with a rental contact taking precedence in snapshots.
    pub contact: Option<Contact>,
    /// Devices and computers in main-router-first, parent-before-child order.
    pub nodes: Vec<LanNodeLayout>,
    pub admin: Option<NetworkAdmin>,
    /// Whether the network is disabled for nonpayment (`NetworkLan.disabledNetwork`).
    pub disabled: bool,
    /// Connection speed (`NetworkLan.netSpeed`).
    pub net_speed: u8,
    /// Whether a rented-server network hired an administrator (`NetworkLan.superAdminRental`).
    pub rental_admin: bool,
    /// Saved versions selected by this network's router node. Empty chooses with the seeded draw.
    pub library_choices: Vec<(LibraryKind, String)>,
}

/// Device or computer in a [`NetworkLayout`].
#[derive(Debug, Clone)]
pub struct LanNodeLayout {
    /// Parent device index in the layout; `None` for the main router.
    pub parent: Option<usize>,
    /// Computer gateway; empty for devices.
    pub gateway: String,
    pub device: DeviceLayout,
}

/// Node type in a [`NetworkLayout`].
#[derive(Debug, Clone)]
pub enum DeviceLayout {
    /// Router or switch with ports and port forwards.
    Router {
        draft: RouterDraft,
        ports: Vec<Port>,
        forwards: Vec<ForwardLayout>,
    },
    /// Hub without an address.
    Hub,
    /// Computer, marked as the player computer by its draft when applicable.
    Computer(Box<ComputerDraft>),
}

/// Router public port `number` forwarded to `port` on the computer at `lan_ip`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForwardLayout {
    pub number: u16,
    pub lan_ip: String,
    pub port: u16,
}
