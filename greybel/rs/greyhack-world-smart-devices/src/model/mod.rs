//! Smart-appliance, camera, and traffic entities.

pub(crate) mod camera;
pub(crate) mod traffic;

pub use camera::{CameraFeed, CameraNpc};
pub use traffic::{LocatedVehicle, TrafficNet};

/// Transient traffic-camera state owned by the smart-devices domain.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TrafficState {
    /// World second the police traffic cameras work again after an accident, if blocked.
    pub cooldown_until: Option<u64>,
    /// Last car found by `locate_vehicle`, used by accidents (`PlayerHelperServer.npcInfoCar`).
    pub located_vehicle: Option<LocatedVehicle>,
}
