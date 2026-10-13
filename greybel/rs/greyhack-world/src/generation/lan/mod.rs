//! Plans LANs using `NetworkLan`, `RouterNet`, `HubNet`, and `PostGeneration`: routers, switches,
//! hubs, subnets, Wi-Fi, firewalls, computer gateways and services, and main-router forwards. Grey
//! Hack rules use the world random state, making each seed deterministic without reproducing Grey
//! Hack's random stream.

mod computer_plan;
mod device_plan;
mod plan;
mod planner;
mod pool;
mod pool_service;
mod service_plan;

pub(crate) use computer_plan::ComputerPlan;
pub(crate) use device_plan::DevicePlan;
use greyhack_behaviour::NetworkKind;
pub(crate) use plan::LanPlan;
use planner::Planner;
use pool::Pool;
use pool_service::PoolService;
pub(crate) use service_plan::ServicePlan;

use crate::Rng;
use crate::generation::missions::MissionRequest;

/// Plans a LAN of `kind` with its main router at `router_ip`.
pub(crate) fn plan(
    rng: &mut Rng,
    kind: NetworkKind,
    router_ip: &str,
    mission: Option<MissionRequest>,
) -> LanPlan {
    Planner::new(rng, kind, router_ip, mission).run()
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
