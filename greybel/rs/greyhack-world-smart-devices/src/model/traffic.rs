//! Network traffic-camera state.

use greyhack_world_core::{NetworkId, PersonId};

/// Main-router traffic-camera state: the NPC workstation account used by police camera systems
/// (`NetworkLan.trafficCredentials`) and the traffic override
/// (`ConfigOS.IsOverrideSettingsEnabled`, `IsOverrideUsed`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TrafficNet {
    /// NPC selected from police roles when available; otherwise the administrator.
    pub credentials: Option<PersonId>,
    /// Traffic-lights weakness enabled the override.
    pub override_enabled: bool,
    /// An accident consumed the override.
    pub override_used: bool,
}

/// Car last found by `locate_vehicle` on police network `police`: NPC `driver`
/// (`PlayerHelperServer.SetNpcInfoCar`). An accident involving this car kills the driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocatedVehicle {
    /// NPC driving the located car.
    pub driver: PersonId,
    /// Police network through which the car was found.
    pub police: NetworkId,
}
