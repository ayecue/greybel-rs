//! Smart-appliance, traffic-camera, and CCTV services.

pub(crate) mod appliance;
pub(crate) mod cctv;
pub(crate) mod traffic;

pub use appliance::{ApplianceFailure, Appliances};
pub use cctv::{Cctv, CctvFailure};
pub use traffic::{Traffic, TrafficFailure};
