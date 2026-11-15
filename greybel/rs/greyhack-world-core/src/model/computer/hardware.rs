//! Installed computer hardware (`Hardware`). Parts are shared rows with common `ItemHardware`
//! fields. Installation rows store per-computer health and network-card state.

/// RAM of the standard hardware of generated computers and the player computer.
pub const STANDARD_RAM_MB: u32 = 2048;
/// Disk capacity of the standard hardware of generated computers and the player computer.
pub const STANDARD_DISK_MB: f32 = 10_000.0;

/// Common hardware fields (`ItemHardware`); `health` belongs to the installation.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    pub name: String,
    pub quality: u32,
    pub required_power: f32,
    pub health: f32,
}

impl Part {
    /// Creates an undamaged part named `name`.
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            quality: 5,
            health: 100.0,
            required_power: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Motherboard {
    pub part: Part,
    pub cpu_slots: u32,
    pub cpu_socket: String,
    pub ram_model: u32,
    pub ram_slots: u32,
    pub max_ram_mb: u32,
    pub pci_slots: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PowerSupply {
    pub part: Part,
    pub power: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cpu {
    pub part: Part,
    pub speed: f32,
    pub cores: u32,
    pub socket: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ram {
    pub part: Part,
    pub memory_mb: u32,
    pub model: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Gpu {
    pub part: Part,
    pub hashrate: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HardDisk {
    pub part: Part,
    pub speed: u32,
    pub size_mb: f32,
}

/// Network card (`Hardware.NetworkDevice`). The active card connects the computer; `active` and
/// `monitor_enabled` belong to the installation.
#[derive(Debug, Clone, PartialEq)]
pub struct NetCard {
    pub part: Part,
    pub wifi: bool,
    pub monitor_support: bool,
    pub monitor_enabled: bool,
    pub active: bool,
}

impl NetCard {
    /// Creates an inactive card with monitor mode disabled.
    #[must_use]
    pub fn new(wifi: bool, monitor_support: bool) -> Self {
        Self {
            part: Part::new(if wifi { "Wifi card" } else { "Ethernet card" }),
            wifi,
            monitor_support,
            monitor_enabled: false,
            active: false,
        }
    }
}

impl NetCard {
    /// Returns interface names for `cards` in order ([`greyhack_behaviour::interface_names`]).
    #[must_use]
    pub fn interface_names(cards: &[Self]) -> Vec<String> {
        greyhack_behaviour::interface_names(cards.iter().map(|card| card.wifi))
    }

    /// Returns the index of the card named `interface` in `cards`.
    #[must_use]
    pub fn position(cards: &[Self], interface: &str) -> Option<usize> {
        Self::interface_names(cards)
            .iter()
            .position(|name| name == interface)
    }
}

/// Mining parts assigned by administrator and generation options.
#[derive(Debug, Clone, PartialEq)]
pub struct MinerHardware {
    pub cpu_speed: f32,
    pub cpu_cores: u32,
    pub ram_model: u32,
    pub gpu_hashrate: f32,
}

impl Default for MinerHardware {
    fn default() -> Self {
        Self {
            cpu_speed: 1.0,
            cpu_cores: 1,
            ram_model: 1,
            gpu_hashrate: 0.0,
        }
    }
}

/// All hardware installed in a computer.
#[derive(Debug, Clone, PartialEq)]
pub struct Hardware {
    pub motherboard: Option<Motherboard>,
    pub power_supply: Option<PowerSupply>,
    pub cpus: Vec<Cpu>,
    pub rams: Vec<Ram>,
    pub gpus: Vec<Gpu>,
    pub hard_disks: Vec<HardDisk>,
    pub net_cards: Vec<NetCard>,
}

impl Hardware {
    /// Creates one of each part: `ram_mb` memory, `disk_mb` disk, requested `mining` parts, and
    /// `net_cards`.
    #[must_use]
    pub fn standard(
        ram_mb: u32,
        disk_mb: f32,
        miner_hardware: &MinerHardware,
        net_cards: Vec<NetCard>,
    ) -> Self {
        Self {
            motherboard: Some(Motherboard {
                part: Part::new("Motherboard"),
                cpu_slots: 1,
                cpu_socket: "LGA1151".to_owned(),
                ram_model: miner_hardware.ram_model,
                ram_slots: 2,
                max_ram_mb: ram_mb.max(1024) * 2,
                pci_slots: 2,
            }),
            power_supply: Some(PowerSupply {
                part: Part::new("Power supply"),
                power: 500,
            }),
            cpus: vec![Cpu {
                part: Part::new("CPU"),
                speed: miner_hardware.cpu_speed,
                cores: miner_hardware.cpu_cores,
                socket: "LGA1151".to_owned(),
            }],
            rams: vec![Ram {
                part: Part::new("RAM"),
                memory_mb: ram_mb,
                model: miner_hardware.ram_model,
            }],
            gpus: if miner_hardware.gpu_hashrate > 0.0 {
                vec![Gpu {
                    part: Part::new("GPU"),
                    hashrate: miner_hardware.gpu_hashrate,
                }]
            } else {
                Vec::new()
            },
            hard_disks: vec![HardDisk {
                part: Part::new("Hard disk"),
                speed: 100,
                size_mb: disk_mb,
            }],
            net_cards,
        }
    }

    /// Total RAM capacity.
    #[must_use]
    pub fn ram_mb(&self) -> u32 {
        self.rams.iter().map(|ram| ram.memory_mb).sum()
    }

    /// Total disk capacity.
    #[must_use]
    pub fn disk_capacity_mb(&self) -> f32 {
        self.hard_disks.iter().map(|disk| disk.size_mb).sum()
    }

    /// Mining-round hash power ([`greyhack_behaviour::hash_power`]).
    #[must_use]
    pub fn mine_power(&self) -> f32 {
        greyhack_behaviour::hash_power(
            self.cpus.iter().map(|cpu| (cpu.speed, cpu.cores)),
            self.rams.iter().map(|ram| (ram.memory_mb, ram.model)),
            self.gpus.iter().map(|gpu| gpu.hashrate),
        )
    }
}
