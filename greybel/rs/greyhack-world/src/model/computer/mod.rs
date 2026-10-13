//! Device kinds, processes, ports, and generation drafts.

use greyhack_behaviour::{
    DeviceKind, FIRST_SPAWN_PID, NetworkKind, PASSWD_FOLDER, PASSWD_PATH, ServiceId,
};

use self::account_files::{etc_folder, passwd_file_draft, passwd_is_reusable, passwd_text};
use crate::RouterId;
use crate::model::{FileSystemDraft, NetworkId, PersonId, User};

pub(crate) mod account_files;
pub(crate) mod camera;
pub(crate) mod failure;
pub(crate) mod filesystem;
pub(crate) mod hardware;
pub(crate) mod saved_network;
pub(crate) mod startup;
pub(crate) mod user;

pub use hardware::{
    Cpu, Gpu, HardDisk, Hardware, MiningHardware, Motherboard, NetCard, Part, PowerSupply, Ram,
};
pub(crate) use hardware::{STANDARD_DISK_MB, STANDARD_RAM_MB};
pub use saved_network::{SavedNetwork, SavedWifiFailure};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComputerId(pub(crate) u32);

#[derive(Debug, Clone, PartialEq)]
pub struct Process {
    pub pid: u32,
    pub user: String,
    pub name: String,
    pub cpu: f32,
    pub ram_mb: f32,
    pub protected: bool,
    /// Service owned by this daemon.
    pub service: Option<ServiceId>,
    pub is_terminal: bool,
}

/// Service port. Its version matches the service library version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Port {
    pub number: u16,
    pub service: ServiceId,
    pub version: String,
    pub closed: bool,
    pub visible: bool,
}

/// Router public port `number` forwarded to `port` on `computer` at `lan_ip`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortForward {
    pub number: u16,
    pub computer: ComputerId,
    pub lan_ip: String,
    pub port: Port,
}

/// Location of a device or computer in a LAN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanPlace {
    /// Parent device.
    pub parent: ComputerId,
    /// Router used for connectivity; `None` for devices.
    pub router: Option<RouterId>,
    /// Gateway LAN address; empty for devices.
    pub gateway: String,
}

impl LanPlace {
    /// Places a device under `parent`.
    #[must_use]
    pub const fn device_under(parent: ComputerId) -> Self {
        Self {
            parent,
            router: None,
            gateway: String::new(),
        }
    }

    /// Places a computer under `parent`, using `gateway` and connecting through `router`.
    #[must_use]
    pub fn computer_under(parent: ComputerId, router: RouterId, gateway: &str) -> Self {
        Self {
            parent,
            router: Some(router),
            gateway: gateway.to_owned(),
        }
    }
}

/// Device row. Hardware, users, processes, ports, and files are stored separately.
#[derive(Debug, Clone, PartialEq)]
pub struct Computer {
    pub id: ComputerId,
    pub kind: DeviceKind,
    pub network: NetworkId,
    /// Parent LAN device (`Device.lanPcInfos`, `hubs`, `routers`); `None` for the network main
    /// router.
    pub parent: Option<ComputerId>,
    /// Connection router (`Computer.router`): the main router for generated computers, or the
    /// Wi-Fi/Ethernet router for connected computers; `None` for devices.
    pub router: Option<RouterId>,
    /// Gateway LAN address (`LanPcInfo.gateway`), empty for devices. Hub computers use their
    /// adopting router.
    pub gateway: String,
    pub name: String,
    pub local_ip: String,
    pub next_pid: u32,
    pub reboot_end: u64,
    /// Permanently unreachable flag (`LanPcInfo.isHidden`), as defined by a copied Grey Hack
    /// network layout.
    pub hidden: bool,
    /// NPC smartphone owner (`LanPcInfo.npcOwner`). The phone is unreachable while the NPC is not
    /// at work.
    pub npc_owner: Option<PersonId>,
    /// Original network kind (`Computer.tipoRed`), retained after joining another network.
    pub network_kind: NetworkKind,
}

impl Computer {
    #[must_use]
    pub fn is_rebooting(&self, now: u64) -> bool {
        now < self.reboot_end
    }
}

/// Computer draft used before database insertion.
#[derive(Debug, Clone, PartialEq)]
pub struct ComputerDraft {
    pub kind: DeviceKind,
    /// Player computers receive first-install files.
    pub is_player: bool,
    pub name: String,
    pub local_ip: String,
    pub users: Vec<User>,
    pub processes: Vec<Process>,
    pub ports: Vec<Port>,
    pub next_pid: u32,
    pub filesystem: FileSystemDraft,
    pub hardware: Hardware,
    pub active_users: Vec<String>,
    pub smart: Option<crate::model::SmartApplianceSettings>,
    pub reboot_end: u64,
    /// Permanent unreachability, equivalent to [`Computer::hidden`].
    pub hidden: bool,
    /// NPC smartphone owner, equivalent to [`Computer::npc_owner`].
    pub npc_owner: Option<PersonId>,
    /// Original network kind, equivalent to [`Computer::network_kind`]. Uses the destination
    /// network kind when `None`.
    pub network_kind: Option<NetworkKind>,
    /// Wi-Fi networks the computer connected to, the last the most recent
    /// (`ConfigOS.savedNetworks`).
    pub saved_networks: Vec<SavedNetwork>,
}

impl ComputerDraft {
    #[must_use]
    pub fn new(name: &str, local_ip: &str) -> Self {
        Self {
            kind: DeviceKind::Computer,
            is_player: false,
            name: name.to_owned(),
            local_ip: local_ip.to_owned(),
            users: Vec::new(),
            processes: Vec::new(),
            ports: Vec::new(),
            next_pid: FIRST_SPAWN_PID,
            filesystem: FileSystemDraft::new(),
            hardware: Hardware::standard(
                STANDARD_RAM_MB,
                STANDARD_DISK_MB,
                &MiningHardware::default(),
                Vec::new(),
            ),
            active_users: Vec::new(),
            smart: None,
            reboot_end: 0,
            hidden: false,
            npc_owner: None,
            network_kind: None,
            saved_networks: Vec::new(),
        }
    }

    /// Writes the `/etc/passwd` of the draft users ([`passwd_text`]).
    pub fn write_passwd(&mut self, recreate: bool) {
        let content = passwd_text(&self.users);
        let fs = &mut self.filesystem;
        let passwd_file = fs.file(PASSWD_PATH, true);
        let file = match passwd_file {
            Some(file) if passwd_is_reusable(fs.node(file).data(), recreate) => file,
            _ => {
                if let Some(file) = passwd_file {
                    fs.remove(file);
                }

                let root = fs.root();
                let etc = match fs.folder(PASSWD_FOLDER, true) {
                    Some(etc) => etc,
                    None => fs.add(root, etc_folder()),
                };

                fs.add(etc, passwd_file_draft())
            }
        };

        if let Some(data) = fs.node_mut(file).data_mut() {
            data.set_content(&content);
        }
    }
}
