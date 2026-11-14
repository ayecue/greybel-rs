//! Plans LANs using `NetworkLan`, `RouterNet`, `HubNet`, and `PostGeneration`: routers, switches,
//! hubs, subnets, Wi-Fi, firewalls, computer gateways and services, and main-router forwards. Grey
//! Hack rules use the world random state, making each seed deterministic without reproducing Grey
//! Hack's random stream.

pub(crate) mod computer_plan;
pub(crate) mod device_plan;
pub(crate) mod plan;
pub(crate) mod planner;
pub(crate) mod pool;
pub(crate) mod pool_service;
pub(crate) mod service_plan;

pub use computer_plan::ComputerPlan;
pub use device_plan::DevicePlan;
use greyhack_behaviour::{NetworkKind, WorldSeed};
use greyhack_world_missions::MissionRequest;
pub use plan::LanPlan;
use planner::Planner;
use pool::Pool;
use pool_service::PoolService;
pub use service_plan::ServicePlan;

use crate::Rng;

/// Plans a LAN of `kind` with its main router at `router_ip`.
pub(crate) fn plan(
    rng: &mut Rng,
    world_seed: &WorldSeed,
    public_ip: &str,
    kind: NetworkKind,
    router_ip: &str,
    mission: Option<MissionRequest>,
) -> LanPlan {
    Planner::new(rng, world_seed, public_ip, kind, router_ip, mission).run()
}

#[cfg(test)]
#[path = "tests/mod.rs"]
pub(crate) mod tests;
