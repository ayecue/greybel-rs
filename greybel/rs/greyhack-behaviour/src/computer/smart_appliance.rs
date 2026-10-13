//! Settings and breakage of smart appliances (`SmartAppConfigOS`).

/// Settings of a computer-attached smart fridge or microwave (`SmartAppConfigOS`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartApplianceSettings {
    pub model: String,
    pub required_power: i32,
    pub required_temperature: i32,
    pub power: i32,
    pub temperature: i32,
    pub alarm: bool,
    pub override_enabled: bool,
    /// World second when hardware recovers from an accident (`SmartAppConfigOS.brokenUntil`); 0
    /// means never broken.
    pub recovery_at: u64,
}

impl SmartApplianceSettings {
    /// Creates settings of `model` running at the required power and temperature, with the alarm
    /// on.
    #[must_use]
    pub fn new(model: &str, required_power: i32, required_temperature: i32) -> Self {
        Self {
            model: model.to_owned(),
            required_power,
            required_temperature,
            power: required_power,
            temperature: required_temperature,
            alarm: true,
            override_enabled: false,
            recovery_at: 0,
        }
    }

    /// Returns whether hardware is broken at world second `now` (`SmartAppConfigOS.IsBroken`).
    #[must_use]
    pub const fn is_broken(&self, now: u64) -> bool {
        now < self.recovery_at
    }

    /// Returns whether override values match model requirements
    /// (`SmartAppConfigOS.HasOverrideSettingsValues`).
    #[must_use]
    pub const fn has_required_values(&self) -> bool {
        self.power == self.required_power && self.temperature == self.required_temperature
    }

    /// Breaks hardware until `until`, enables its alarm, zeroes values, and locks override settings
    /// (`SmartAppConfigOS.SetBroken`).
    pub const fn break_until(&mut self, until: u64) {
        self.recovery_at = until;
        self.alarm = true;
        self.power = 0;
        self.temperature = 0;
        self.override_enabled = false;
    }
}
