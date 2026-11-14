//! Smart-appliance, traffic-network, and traffic-cooldown persistence.

mod repository;
pub(crate) mod schema;

pub(crate) use repository::SmartDeviceRepository;
