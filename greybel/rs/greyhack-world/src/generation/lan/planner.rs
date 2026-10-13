use greyhack_behaviour::{DeviceKind, NetworkKind, ROUTER_PORT, ServiceId};

use self::addresses::parse;
use super::{DevicePlan, LanPlan, PoolService};
use crate::generation::missions::{MissionPlacement, MissionRequest};
use crate::{ForwardLayout, Rng};

mod addresses;
mod computers;
mod devices;
mod mission;
mod post_generation;
mod services;

/// References a planned computer by device index and per-device computer index.
type ComputerRef = (usize, usize);

/// LAN planner state (`NetworkLan.GlobalLan`).
pub(super) struct Planner<'r> {
    rng: &'r mut Rng,
    kind: NetworkKind,
    max_devices: u32,
    total_devices: u32,
    devices: Vec<DevicePlan>,
    /// Main-router port table: configuration port followed by forwards.
    table: Vec<ForwardLayout>,
    /// Allocated subnets (`privateIps`), formatted as `a.b.c.0`.
    subnets: Vec<String>,
    /// Cursor for allocating the next address (`lanIp`).
    cursor: [u32; 4],
    last_device: usize,
    public: Vec<PoolService>,
    private: Vec<PoolService>,
    /// Private-subnet gateway computers awaiting routers, with their hub computer counts.
    pending_gateways: Vec<(ComputerRef, u32)>,
    /// Hub computers awaiting adoption by a router.
    pending_hub: Vec<ComputerRef>,
    /// Devices whose firewalls deny traffic by default.
    deny_devices: Vec<usize>,
    /// Router computer addresses whose existing forwards must not follow readdressing.
    router_forward_addresses: Vec<String>,
    cameras: u32,
    fridge: bool,
    microwave: bool,
    /// The mission of the network, until `NetworkLan.CreateMission` places it.
    mission: Option<MissionRequest>,
    placement: Option<MissionPlacement>,
}

/// Maximum generated device count for network `kind` (`NetworkLan.ConfigMaxDevices`,
/// `GlobalLan.MaxDevicesGen` for normal-access networks).
const fn max_devices(kind: NetworkKind) -> u32 {
    match kind {
        NetworkKind::RentServer => 0,
        NetworkKind::Police
        | NetworkKind::University
        | NetworkKind::Bank
        | NetworkKind::Neurobox => 20,
        NetworkKind::MailService
        | NetworkKind::HackShop
        | NetworkKind::ComputerShop
        | NetworkKind::HardwareManufacturer
        | NetworkKind::CurrencyCreation => 10,
        _ => 5,
    }
}

impl<'r> Planner<'r> {
    pub(super) fn new(
        rng: &'r mut Rng,
        kind: NetworkKind,
        router_ip: &str,
        mission: Option<MissionRequest>,
    ) -> Self {
        let router = parse(router_ip);

        Self {
            max_devices: max_devices(kind),
            total_devices: 0,
            devices: vec![DevicePlan::new(DeviceKind::Router, router_ip, None)],
            table: vec![ForwardLayout {
                number: ROUTER_PORT,
                lan_ip: router_ip.to_owned(),
                port: ROUTER_PORT,
            }],
            subnets: vec![format!("{}.{}.{}.0", router[0], router[1], router[2])],
            cursor: router,
            last_device: 0,
            public: Vec::new(),
            private: [ServiceId::Ssh, ServiceId::Ftp]
                .into_iter()
                .map(|service| PoolService {
                    service,
                    chance: 0,
                    closed: false,
                })
                .collect(),
            pending_gateways: Vec::new(),
            pending_hub: Vec::new(),
            deny_devices: Vec::new(),
            router_forward_addresses: Vec::new(),
            cameras: 0,
            fridge: false,
            microwave: false,
            mission,
            placement: None,
            rng,
            kind,
        }
    }

    /// Returns the main-router-first plan and forwards, excluding the router configuration port.
    pub(super) fn run(mut self) -> LanPlan {
        self.main_router();
        self.post_generation();

        LanPlan {
            devices: self.devices,
            forwards: self.table.into_iter().skip(1).collect(),
            mission: self.placement,
        }
    }
}

impl Planner<'_> {
    /// `rng.Next(min, max)`: returns `min <= value < max`.
    fn next(&mut self, min: u32, max: u32) -> u32 {
        if max <= min {
            min
        } else {
            self.rng.range(min, max - 1)
        }
    }
}
