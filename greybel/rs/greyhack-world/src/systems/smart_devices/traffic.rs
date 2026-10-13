//! Police traffic cameras (`TrafficNetIntrinsics`): access credentials, vehicle lookup during
//! commutes, and fatal accidents (`PlayerHelperServer.EnableCarAccidentServerRpc`).

use greyhack_behaviour::{
    ACCIDENT_COOLDOWN_SECONDS, NetworkKind, license_plate_address, normalize_license_plate,
};

use crate::systems::smart_devices::appliance::ConnectionLoss;
use crate::{
    ComputerId, JobRole, LocatedVehicle, NetworkId, Npc, PersonId, Result, Rng, TrafficNet, World,
};

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
    /// The camera device's computer is gone.
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

impl World {
    /// Selects the workstation account for police traffic cameras
    /// (`NetworkLan.SetTrafficCredentials`): a police-role NPC if available, otherwise the
    /// administrator.
    pub(in crate::systems) async fn select_traffic_credentials(
        &mut self,
        network: NetworkId,
    ) -> Result<()> {
        let Some(row) = self.db.network(network).await? else {
            return Ok(());
        };

        if row.kind != NetworkKind::Police {
            return Ok(());
        }

        let public_ip = self.network_public_ip(network).await?;
        let mut rng = Rng::stable(self.seed, &[&public_ip, "traffic credentials"]);
        let police: Vec<PersonId> = self
            .db
            .npcs_in(network)
            .await?
            .into_iter()
            .filter(|npc| JobRole::POLICE.contains(&npc.job_role))
            .map(|npc| npc.person)
            .collect();
        let credentials = if police.is_empty() {
            self.db.network_admin_person(network).await?
        } else {
            Some(*rng.pick(&police))
        };
        let mut traffic = self.db.traffic(network).await?;

        traffic.credentials = credentials;

        self.db.set_traffic(network, &traffic).await
    }

    /// Returns `network` traffic-camera state.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn traffic(&self, network: NetworkId) -> Result<TrafficNet> {
        self.db.traffic(network).await
    }

    /// Updates `network` traffic-camera state.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn save_traffic(&mut self, network: NetworkId, traffic: &TrafficNet) -> Result<()> {
        self.db.set_traffic(network, traffic).await
    }

    /// Returns the police NPC whose workstation opens `network` cameras, or its administrator
    /// (`NetworkLan.GetNpcInfoTrafficCrecentials`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn traffic_credentials(&self, network: NetworkId) -> Result<Option<Npc>> {
        let person = match self.db.traffic(network).await?.credentials {
            Some(person) => Some(person),
            None => self.db.network_admin_person(network).await?,
        };

        Ok(match person {
            Some(person) => self.db.npc(person).await?,
            None => None,
        })
    }

    /// Returns the network of camera device `computer` when it can be used
    /// (`TrafficNetIntrinsics.CheckStatus`): the computer exists, has a router, and is on a police
    /// network.
    async fn camera_network(
        &self,
        computer: ComputerId,
    ) -> Result<std::result::Result<NetworkId, TrafficFailure>> {
        let device = match self.connected_computer(computer).await? {
            Ok(device) => device,
            Err(failure) => return Ok(Err(failure.into())),
        };

        if device.network_kind != NetworkKind::Police {
            return Ok(Err(TrafficFailure::MissingPoliceNetwork));
        }

        Ok(Ok(device.network))
    }

    /// Checks global camera access from camera device `computer`
    /// (`TrafficNetIntrinsics.camera_link_system`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
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

    /// Returns the NPC whose account opens the cameras of camera device `computer`
    /// (`TrafficNetIntrinsics.get_credentials_info`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
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

    /// Finds `plate` through camera device `computer` (`TrafficNetIntrinsics.locate_vehicle`); see
    /// [`World::locate_vehicle`].
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn locate_vehicle_from(
        &mut self,
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

    /// Finds `plate` on `police` cameras using the camera-access password
    /// (`TrafficNetIntrinsics.locate_vehicle`). Driver must be alive and commuting. With `record`,
    /// stores the car for the next accident (`PlayerHelperServer.SetNpcInfoCar`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn locate_vehicle(
        &mut self,
        police: NetworkId,
        plate: &str,
        password: &str,
        record: bool,
    ) -> Result<Result<(), TrafficFailure>> {
        if self.has_traffic_cooldown() {
            return Ok(Err(TrafficFailure::Cooldown));
        }

        if self.db.traffic(police).await?.override_used {
            return Ok(Err(TrafficFailure::CamerasOff));
        }

        let Some(credentials) = self.traffic_credentials(police).await? else {
            return Ok(Err(TrafficFailure::MissingWorkstation(String::new())));
        };

        if !self.db.has_computer(credentials.computer).await? {
            return Ok(Err(TrafficFailure::MissingWorkstation(credentials.name)));
        }

        let opens = self
            .db
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
        let address = license_plate_address(&self.world_seed, &plate);
        let driver = match self.db.router_at(&address).await? {
            Some(router) => self
                .db
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
            && self.is_crown_shield_network(driver.computer).await?
        {
            return Ok(Err(TrafficFailure::CrownShield));
        }

        if driver.is_dead_or_fired(self.time_seconds)
            || !driver.schedule.is_commuting(self.time_seconds)
        {
            return Ok(Err(TrafficFailure::OutOfSight));
        }

        if record {
            self.located_vehicle = Some(LocatedVehicle {
                driver: driver.person,
                police,
            });
        }

        Ok(Ok(()))
    }

    /// Triggers an accident with the last located car
    /// (`PlayerHelperServer.EnableCarAccidentServerRpc`). If no cooldown is active, the driver is
    /// commuting, and the police override is enabled and unused, consumes the override, kills the
    /// driver, and starts a 12-hour cooldown. Returns whether the driver died.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn traffic_accident(&mut self) -> Result<bool> {
        let Some(car) = self.located_vehicle else {
            return Ok(false);
        };

        if self.has_traffic_cooldown() {
            return Ok(false);
        }

        let Some(driver) = self.db.npc(car.driver).await? else {
            return Ok(false);
        };

        if driver.job_role == JobRole::Admin
            && self.is_crown_shield_network(driver.computer).await?
        {
            return Ok(false);
        }

        if !driver.schedule.is_commuting(self.time_seconds) {
            return Ok(false);
        }

        let mut traffic = self.db.traffic(car.police).await?;

        if !traffic.override_enabled || traffic.override_used {
            return Ok(false);
        }

        traffic.override_used = true;
        self.db.set_traffic(car.police, &traffic).await?;
        self.set_dead(&driver).await?;
        self.traffic_cooldown_until = Some(self.time_seconds + ACCIDENT_COOLDOWN_SECONDS);

        Ok(true)
    }

    /// Returns whether an accident cooldown remains active.
    #[must_use]
    pub fn has_traffic_cooldown(&self) -> bool {
        self.traffic_cooldown_until
            .is_some_and(|until| self.time_seconds < until)
    }
}

#[cfg(test)]
#[path = "tests/traffic.rs"]
mod tests;
