//! Smart appliances (`SmartApplianceIntrinsics`): availability, override settings, the alarm, and
//! the accidents they trigger.

use crate::{Computer, ComputerId, Result, SmartApplianceSettings, World};

/// Accident recovery duration in world seconds (`NetworkLan.HelperAccident.GetRecoveryTime`).
const RECOVERY_SECONDS: u64 = 24 * 60 * 60;

/// Why a device cannot be used over the network.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConnectionLoss {
    /// The computer is gone.
    ComputerUnavailable,
    /// The computer has no router.
    NoInternet,
}

/// Why a smart-appliance operation failed (`SmartApplianceIntrinsics`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplianceFailure {
    /// The appliance's computer is gone.
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

impl World {
    /// Returns smart-appliance settings for `computer`, if applicable.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn smart(&self, computer: ComputerId) -> Result<Option<SmartApplianceSettings>> {
        self.db.smart(computer).await
    }

    /// Updates or removes smart-appliance settings for `computer`.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn save_smart(
        &mut self,
        computer: ComputerId,
        smart: Option<&SmartApplianceSettings>,
    ) -> Result<()> {
        self.db.set_smart(computer, smart).await
    }

    /// Returns `computer` when it exists and has a router.
    pub(crate) async fn connected_computer(
        &self,
        computer: ComputerId,
    ) -> Result<std::result::Result<Computer, ConnectionLoss>> {
        let Some(device) = self.computer(computer).await? else {
            return Ok(Err(ConnectionLoss::ComputerUnavailable));
        };

        if device.router.is_none() {
            return Ok(Err(ConnectionLoss::NoInternet));
        }

        Ok(Ok(device))
    }

    /// Returns the settings of the appliance on `computer` when it can be used.
    async fn usable_appliance(
        &self,
        computer: ComputerId,
    ) -> Result<std::result::Result<SmartApplianceSettings, ApplianceFailure>> {
        let Some(smart) = self.smart(computer).await? else {
            return Ok(Err(ApplianceFailure::ComputerUnavailable));
        };

        if let Err(failure) = self.connected_computer(computer).await? {
            return Ok(Err(failure.into()));
        }

        if smart.is_broken(self.time_seconds()) {
            return Ok(Err(ApplianceFailure::Malfunction));
        }

        Ok(Ok(smart))
    }

    /// Makes `computer` a smart appliance of `settings`. Returns `false` if it is gone.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn enable_smart_appliance(
        &mut self,
        computer: ComputerId,
        settings: &SmartApplianceSettings,
    ) -> Result<bool> {
        if self.computer(computer).await?.is_none() {
            return Ok(false);
        }

        self.save_smart(computer, Some(settings)).await?;

        Ok(true)
    }

    /// Returns the model of the appliance on `computer` (`SmartApplianceIntrinsics.model`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn appliance_model(
        &self,
        computer: ComputerId,
    ) -> Result<std::result::Result<String, ApplianceFailure>> {
        Ok(self
            .usable_appliance(computer)
            .await?
            .map(|smart| smart.model))
    }

    /// Sets override power and temperature when they match the model's required values
    /// (`SmartApplianceIntrinsics.override_settings`). An accident occurs when the alarm is off.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn override_appliance(
        &mut self,
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
        self.save_smart(computer, Some(&smart)).await?;

        if smart.alarm {
            return Ok(Err(ApplianceFailure::AlarmWarning));
        }

        self.trigger_accident(computer).await?;

        Ok(Ok(()))
    }

    /// Sets the appliance alarm (`SmartApplianceIntrinsics.set_alarm`). An accident occurs when the
    /// alarm is off and override values are set.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn set_appliance_alarm(
        &mut self,
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
        self.save_smart(computer, Some(&smart)).await?;
        self.trigger_accident(computer).await?;

        Ok(Ok(()))
    }

    /// Triggers an accident when the alarm is off and required override values are set
    /// (`SmartApplianceIntrinsics.TriggerAccidentIfReady`). NPCs reaching the appliance within five
    /// minutes die, and the appliance remains broken for one day. The network accident also blocks
    /// cameras during recovery (`NetworkLan.BlockAccessCameras`).
    async fn trigger_accident(&mut self, computer: ComputerId) -> Result<()> {
        let Some(mut smart) = self.smart(computer).await? else {
            return Ok(());
        };

        if smart.alarm || !smart.has_required_values() {
            return Ok(());
        }

        self.kill_npcs_at_appliance(computer).await?;
        smart.break_until(self.time_seconds() + RECOVERY_SECONDS);
        self.save_smart(computer, Some(&smart)).await?;

        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/appliance.rs"]
mod tests;
