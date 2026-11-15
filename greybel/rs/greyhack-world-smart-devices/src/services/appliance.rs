//! Smart appliances (`SmartApplianceIntrinsics`): availability, override settings, the alarm, and
//! the accidents they trigger.

use greyhack_behaviour::SmartApplianceSettings;
use greyhack_world_core::{Computer, ComputerId, Result};

/// Accident recovery duration in world seconds (`NetworkLan.HelperAccident.GetRecoveryTime`).
const RECOVERY_SECONDS: u64 = 24 * 60 * 60;

/// Why a device cannot be used over the network.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionLoss {
    /// The computer no longer exists.
    ComputerUnavailable,
    /// The computer has no router.
    NoInternet,
}

/// Why a smart-appliance operation failed (`SmartApplianceIntrinsics`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplianceFailure {
    /// The appliance's computer no longer exists.
    ComputerUnavailable,
    /// The appliance has no internet access.
    NoInternet,
    /// The hardware is broken after an accident.
    Malfunction,
    /// The override settings are locked.
    SettingsLock,
    /// The override's power or temperature is not the model's.
    InvalidSettings,
    /// Override is set while the alarm is on, so the appliance stops before reaching the values.
    AlarmWarning,
}

impl From<ConnectionLoss> for ApplianceFailure {
    fn from(failure: ConnectionLoss) -> Self {
        match failure {
            ConnectionLoss::ComputerUnavailable => Self::ComputerUnavailable,
            ConnectionLoss::NoInternet => Self::NoInternet,
        }
    }
}

impl ApplianceService {
    /// Returns the smart-appliance settings of `computer`, if it is an appliance.
    pub async fn smart(&self, computer: ComputerId) -> Result<Option<SmartApplianceSettings>> {
        self.smart_devices.smart(computer).await
    }

    /// Sets or clears the smart-appliance settings of `computer`.
    pub async fn save_smart(
        &self,
        computer: ComputerId,
        smart: Option<&SmartApplianceSettings>,
    ) -> Result<()> {
        self.smart_devices.set_smart(computer, smart).await
    }

    /// Returns `computer` when it exists and has a router.
    pub async fn connected_computer(
        &self,
        computer: ComputerId,
    ) -> Result<std::result::Result<Computer, ConnectionLoss>> {
        let Some(device) = self.networks.topology.computer(computer).await? else {
            return Ok(Err(ConnectionLoss::ComputerUnavailable));
        };

        if device.router.is_none() {
            return Ok(Err(ConnectionLoss::NoInternet));
        }

        Ok(Ok(device))
    }

    /// Enables `settings` on an existing computer and returns whether it was found.
    pub async fn enable_smart_appliance(
        &self,
        computer: ComputerId,
        settings: &SmartApplianceSettings,
    ) -> Result<bool> {
        if self.networks.topology.computer(computer).await?.is_none() {
            return Ok(false);
        }

        self.smart_devices
            .set_smart(computer, Some(settings))
            .await?;

        Ok(true)
    }

    /// Returns the model of a usable appliance (`SmartApplianceIntrinsics.model`).
    pub async fn appliance_model(
        &self,
        computer: ComputerId,
    ) -> Result<std::result::Result<String, ApplianceFailure>> {
        Ok(self
            .usable_appliance(computer)
            .await?
            .map(|smart| smart.model))
    }

    /// Applies the model's required power and temperature (`SmartApplianceIntrinsics.override_settings`).
    pub async fn override_appliance(
        &self,
        computer: ComputerId,
        power: f64,
        temperature: f64,
    ) -> Result<std::result::Result<(), ApplianceFailure>> {
        let mut smart = match self.usable_appliance(computer).await? {
            Ok(smart) => smart,
            Err(failure) => return Ok(Err(failure)),
        };

        if power != f64::from(smart.required_power)
            || temperature != f64::from(smart.required_temperature)
        {
            return Ok(Err(ApplianceFailure::InvalidSettings));
        }

        if !smart.override_enabled {
            return Ok(Err(ApplianceFailure::SettingsLock));
        }

        smart.power = smart.required_power;
        smart.temperature = smart.required_temperature;
        self.smart_devices.set_smart(computer, Some(&smart)).await?;

        if smart.alarm {
            return Ok(Err(ApplianceFailure::AlarmWarning));
        }

        self.trigger_accident(computer).await?;

        Ok(Ok(()))
    }

    /// Enables or disables the appliance alarm (`SmartApplianceIntrinsics.set_alarm`).
    pub async fn set_appliance_alarm(
        &self,
        computer: ComputerId,
        enable: bool,
    ) -> Result<std::result::Result<(), ApplianceFailure>> {
        let mut smart = match self.usable_appliance(computer).await? {
            Ok(smart) => smart,
            Err(failure) => return Ok(Err(failure)),
        };

        if !smart.override_enabled {
            return Ok(Err(ApplianceFailure::SettingsLock));
        }

        smart.alarm = enable;
        self.smart_devices.set_smart(computer, Some(&smart)).await?;
        self.trigger_accident(computer).await?;

        Ok(Ok(()))
    }
}

impl ApplianceService {
    /// Returns an appliance only when it is present, connected, and not broken.
    pub async fn usable_appliance(
        &self,
        computer: ComputerId,
    ) -> Result<std::result::Result<SmartApplianceSettings, ApplianceFailure>> {
        let Some(smart) = self.smart(computer).await? else {
            return Ok(Err(ApplianceFailure::ComputerUnavailable));
        };

        if let Err(failure) = self.connected_computer(computer).await? {
            return Ok(Err(failure.into()));
        }

        if smart.is_broken(self.world.time_seconds()) {
            return Ok(Err(ApplianceFailure::Malfunction));
        }

        Ok(Ok(smart))
    }

    /// Kills NPCs at an appliance and breaks it when its unsafe values trigger an accident.
    pub async fn trigger_accident(&self, computer: ComputerId) -> Result<()> {
        let Some(mut smart) = self.smart(computer).await? else {
            return Ok(());
        };

        if smart.alarm || !smart.has_required_values() {
            return Ok(());
        }

        self.schedules.kill_npcs_at_appliance(computer).await?;
        smart.break_until(self.world.time_seconds() + RECOVERY_SECONDS);
        self.smart_devices.set_smart(computer, Some(&smart)).await?;

        Ok(())
    }
}

/// Smart-appliance settings, overrides, alarms, and accidents.
pub struct ApplianceService {
    pub(crate) world: greyhack_world_core::WorldRef,
    pub(crate) smart_devices: std::rc::Rc<crate::storage::SmartDeviceRepository>,
    pub(crate) networks: greyhack_world_modules::Dep<greyhack_world_core::NetworkService>,
    pub(crate) schedules: greyhack_world_modules::Dep<greyhack_world_npcs::ScheduleService>,
}

impl ApplianceService {
    pub(crate) fn new(
        world: &greyhack_world_core::WorldRef,
        smart_devices: std::rc::Rc<crate::storage::SmartDeviceRepository>,
        networks: greyhack_world_modules::Dep<greyhack_world_core::NetworkService>,
        schedules: greyhack_world_modules::Dep<greyhack_world_npcs::ScheduleService>,
    ) -> Self {
        Self {
            world: world.clone(),
            smart_devices,
            networks,
            schedules,
        }
    }
}
