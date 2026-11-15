//! Police traffic cameras (`TrafficNetIntrinsics`): access credentials, vehicle lookup during
//! commutes, and fatal accidents (`PlayerHelperServer.EnableCarAccidentServerRpc`).

use greyhack_behaviour::{
    ACCIDENT_COOLDOWN_SECONDS, JobRole, NetworkKind, Rng, license_plate_address,
    normalize_license_plate,
};
use greyhack_world_core::{ComputerId, NetworkId, PersonId, Result};
use greyhack_world_npcs::Npc;

use crate::services::appliance::ConnectionLoss;
use crate::{LocatedVehicle, TrafficNet, TrafficState};

impl From<ConnectionLoss> for TrafficFailure {
    fn from(failure: ConnectionLoss) -> Self {
        match failure {
            ConnectionLoss::ComputerUnavailable => Self::ComputerUnavailable,
            ConnectionLoss::NoInternet => Self::NoInternet,
        }
    }
}

/// Vehicle lookup errors (`TrafficNetIntrinsics.locate_vehicle`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrafficFailure {
    /// The camera device's computer no longer exists.
    ComputerUnavailable,
    /// The camera device has no internet access.
    NoInternet,
    /// The camera device is not on a police network.
    MissingPoliceNetwork,
    /// An accident cooldown is active.
    Cooldown,
    /// An accident already consumed the police-network override.
    CamerasOff,
    /// The camera-access NPC computer is missing; contains the NPC full name.
    MissingWorkstation(String),
    /// The camera-access account password is invalid.
    IncorrectPassword,
    /// The plate cannot be decoded into an address and name.
    InvalidPlate,
    /// No NPC with a car has the decoded name on the network at the decoded address.
    UnknownPlate,
    /// The driver is dead or not commuting.
    OutOfSight,
    /// A CrownShield protects the network of an administrator driver.
    CrownShield,
}

impl Traffic {
    /// Selects a police NPC for camera credentials, falling back to the network administrator.
    pub async fn select_traffic_credentials(&self, network: NetworkId) -> Result<()> {
        let Some(row) = self.networks.network(network).await? else {
            return Ok(());
        };

        if row.kind != NetworkKind::Police {
            return Ok(());
        }

        let public_ip = self.networks.topology.network_public_ip(network).await?;
        let mut rng = Rng::stable(self.world.seed(), &[&public_ip, "traffic credentials"]);
        let police: Vec<PersonId> = self
            .npcs
            .npcs_in(network)
            .await?
            .into_iter()
            .filter(|npc| JobRole::POLICE.contains(&npc.job_role))
            .map(|npc| npc.person)
            .collect();
        let credentials = if police.is_empty() {
            self.npcs.network_admin_person(network).await?
        } else {
            Some(*rng.pick(&police))
        };
        let mut traffic = self.smart_devices.traffic(network).await?;

        traffic.credentials = credentials;

        self.smart_devices.set_traffic(network, &traffic).await
    }

    /// Returns the traffic-network state of `network`.
    pub async fn traffic(&self, network: NetworkId) -> Result<TrafficNet> {
        self.smart_devices.traffic(network).await
    }

    /// Writes the traffic-network state of `network`.
    pub async fn save_traffic(&self, network: NetworkId, traffic: &TrafficNet) -> Result<()> {
        self.smart_devices.set_traffic(network, traffic).await
    }

    /// Returns the NPC whose workstation account opens a police network's cameras.
    pub async fn traffic_credentials(&self, network: NetworkId) -> Result<Option<Npc>> {
        let person = match self.smart_devices.traffic(network).await?.credentials {
            Some(person) => Some(person),
            None => self.npcs.network_admin_person(network).await?,
        };

        Ok(match person {
            Some(person) => self.npcs.npc(person).await?,
            None => None,
        })
    }

    /// Checks police-network access and the accident cooldown (`TrafficNetIntrinsics.camera_link_system`).
    pub async fn camera_link_system(
        &self,
        computer: ComputerId,
    ) -> Result<std::result::Result<(), TrafficFailure>> {
        if let Err(failure) = self.camera_network(computer).await? {
            return Ok(Err(failure));
        }

        if self.has_traffic_cooldown() {
            return Ok(Err(TrafficFailure::Cooldown));
        }

        Ok(Ok(()))
    }

    /// Returns camera credentials for the police network containing `computer`.
    pub async fn camera_credentials(
        &self,
        computer: ComputerId,
    ) -> Result<std::result::Result<Option<Npc>, TrafficFailure>> {
        let network = match self.camera_network(computer).await? {
            Ok(network) => network,
            Err(failure) => return Ok(Err(failure)),
        };

        Ok(Ok(self.traffic_credentials(network).await?))
    }

    /// Locates a vehicle through the camera library running on `computer`.
    pub async fn locate_vehicle_from(
        &self,
        computer: ComputerId,
        plate: &str,
        password: &str,
        record: bool,
    ) -> Result<std::result::Result<(), TrafficFailure>> {
        let network = match self.camera_network(computer).await? {
            Ok(network) => network,
            Err(failure) => return Ok(Err(failure)),
        };

        self.locate_vehicle(network, plate, password, record).await
    }

    /// Finds a commuting driver by plate and records the car for a later accident.
    pub async fn locate_vehicle(
        &self,
        police: NetworkId,
        plate: &str,
        password: &str,
        record: bool,
    ) -> Result<Result<(), TrafficFailure>> {
        if self.has_traffic_cooldown() {
            return Ok(Err(TrafficFailure::Cooldown));
        }

        if self.smart_devices.traffic(police).await?.override_used {
            return Ok(Err(TrafficFailure::CamerasOff));
        }

        let Some(credentials) = self.traffic_credentials(police).await? else {
            return Ok(Err(TrafficFailure::MissingWorkstation(String::new())));
        };

        if !self.computers.has_computer(credentials.computer).await? {
            return Ok(Err(TrafficFailure::MissingWorkstation(credentials.name)));
        }

        let opens = self
            .computers
            .user(credentials.computer, &credentials.user)
            .await?
            .is_some_and(|user| user.password == password);

        if !opens {
            return Ok(Err(TrafficFailure::IncorrectPassword));
        }

        let Some(plate) = normalize_license_plate(plate) else {
            return Ok(Err(TrafficFailure::InvalidPlate));
        };
        // `TrafficNetIntrinsics.FindNpcInfoByLicensePlate`: resolves the NPC with this plate on the
        // network at its encoded address.
        let address = license_plate_address(&self.world.world_seed(), &plate);
        let driver = match self.networks.router_at(&address).await? {
            Some(router) => self
                .npcs
                .npcs_in(router.network)
                .await?
                .into_iter()
                .find(|npc| {
                    npc.license_plate
                        .as_deref()
                        .is_some_and(|other| other.eq_ignore_ascii_case(&plate))
                }),
            None => None,
        };
        let Some(driver) = driver else {
            return Ok(Err(TrafficFailure::UnknownPlate));
        };

        if driver.job_role == JobRole::Admin
            && self.guilds.is_crown_shield_network(driver.computer).await?
        {
            return Ok(Err(TrafficFailure::CrownShield));
        }

        if driver.is_dead_or_fired(self.world.time_seconds())
            || !driver.schedule.is_commuting(self.world.time_seconds())
        {
            return Ok(Err(TrafficFailure::OutOfSight));
        }

        if record {
            self.world
                .world_mut()
                .state_mut::<TrafficState>()
                .located_vehicle = Some(LocatedVehicle {
                driver: driver.person,
                police,
            });
        }

        Ok(Ok(()))
    }

    /// Causes the recorded accident when its override, driver, and cooldown permit it.
    pub async fn traffic_accident(&self) -> Result<bool> {
        let Some(car) = self
            .world
            .world_mut()
            .state_mut::<TrafficState>()
            .located_vehicle
        else {
            return Ok(false);
        };

        if self.has_traffic_cooldown() {
            return Ok(false);
        }

        let Some(driver) = self.npcs.npc(car.driver).await? else {
            return Ok(false);
        };

        if driver.job_role == JobRole::Admin
            && self.guilds.is_crown_shield_network(driver.computer).await?
        {
            return Ok(false);
        }

        if !driver.schedule.is_commuting(self.world.time_seconds()) {
            return Ok(false);
        }

        let mut traffic = self.smart_devices.traffic(car.police).await?;

        if !traffic.override_enabled || traffic.override_used {
            return Ok(false);
        }

        traffic.override_used = true;
        self.smart_devices.set_traffic(car.police, &traffic).await?;
        self.jobs.set_dead(&driver).await?;
        self.world
            .world_mut()
            .state_mut::<TrafficState>()
            .cooldown_until = Some(self.world.time_seconds() + ACCIDENT_COOLDOWN_SECONDS);

        Ok(true)
    }

    /// Returns whether an accident cooldown currently blocks traffic-camera operations.
    pub fn has_traffic_cooldown(&self) -> bool {
        self.world
            .world()
            .state_ref::<TrafficState>()
            .and_then(|state| state.cooldown_until)
            .is_some_and(|until| self.world.time_seconds() < until)
    }
}

impl Traffic {
    /// Returns the police network of a connected camera computer.
    pub async fn camera_network(
        &self,
        computer: ComputerId,
    ) -> Result<std::result::Result<NetworkId, TrafficFailure>> {
        let device = match self.appliances.connected_computer(computer).await? {
            Ok(device) => device,
            Err(failure) => return Ok(Err(failure.into())),
        };

        if device.network_kind != NetworkKind::Police {
            return Ok(Err(TrafficFailure::MissingPoliceNetwork));
        }

        Ok(Ok(device.network))
    }
}

impl Traffic {
    /// Loads traffic's in-memory state from its table.
    pub async fn load_state(&self) -> Result<()> {
        let cooldown = self.smart_devices.traffic_cooldown().await?;

        self.world
            .world_mut()
            .state_mut::<TrafficState>()
            .cooldown_until = cooldown;

        Ok(())
    }

    /// Writes traffic's persisted state from the world state map.
    pub async fn save_state(&self) -> Result<()> {
        let cooldown = self
            .world
            .world()
            .state_ref::<TrafficState>()
            .and_then(|state| state.cooldown_until);

        self.smart_devices.set_traffic_cooldown(cooldown).await
    }
}

/// Police traffic cameras, vehicle lookup, and accident state.
pub struct Traffic {
    pub(crate) world: greyhack_world_core::WorldRef,
    pub(crate) smart_devices: std::rc::Rc<crate::storage::SmartDeviceRepository>,
    pub(crate) computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
    pub(crate) networks: greyhack_world_modules::Dep<greyhack_world_core::NetworkService>,
    pub(crate) jobs: greyhack_world_modules::Dep<greyhack_world_npcs::JobService>,
    pub(crate) guilds: greyhack_world_modules::Dep<greyhack_world_guilds::GuildService>,
    pub(crate) appliances: greyhack_world_modules::Dep<crate::ApplianceService>,
    pub(crate) npcs: greyhack_world_modules::Dep<greyhack_world_npcs::NpcService>,
}

impl Traffic {
    pub(crate) fn new(
        registry: &std::rc::Rc<greyhack_world_modules::Registry>,
        smart_devices: std::rc::Rc<crate::storage::SmartDeviceRepository>,
    ) -> Self {
        Self {
            world: registry.world::<greyhack_world_core::WorldRef>().clone(),
            smart_devices,
            computers: registry.get(),
            networks: registry.get(),
            jobs: registry.get(),
            guilds: registry.get(),
            appliances: registry.get(),
            npcs: registry.get(),
        }
    }
}

impl greyhack_world_core::PersistentState for Traffic {
    fn load_state<'a>(
        &'a self,
        _: &'a greyhack_world_core::WorldRef,
    ) -> greyhack_world_effects::BoxFuture<'a, greyhack_world_core::Result<()>> {
        Box::pin(async move { self.load_state().await })
    }

    fn save_state<'a>(
        &'a self,
        _: &'a greyhack_world_core::WorldRef,
    ) -> greyhack_world_effects::BoxFuture<'a, greyhack_world_core::Result<()>> {
        Box::pin(async move { self.save_state().await })
    }
}
