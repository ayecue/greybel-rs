//! Network cards: stored cards and Wi-Fi connections, and what scripts do to them.

use std::rc::Rc;

use greyhack_world_modules::Dep;

use crate::storage::*;
use crate::systems::*;
use crate::*;

pub mod cards;
pub mod script;

/// Network cards, Ethernet, Wi-Fi, and saved networks.
pub struct NetCardService {
    pub(crate) world: WorldRef,
    pub(crate) computer_rows: Rc<ComputerRepository>,
    pub(crate) hardware_rows: Rc<HardwareRepository>,
    pub(crate) router_rows: Rc<RouterRepository>,
    pub(crate) users: Dep<UserService>,
    pub(crate) lan: Dep<NetworkLan>,
    pub(crate) routers: Dep<RouterService>,
    pub(crate) settings: Dep<NetworkSettings>,
    pub(crate) topology: Dep<NetworkTopology>,
}

impl NetCardService {
    pub(crate) fn new(
        registry: &Rc<greyhack_world_modules::Registry>,
        computer_rows: Rc<ComputerRepository>,
        hardware_rows: Rc<HardwareRepository>,
        router_rows: Rc<RouterRepository>,
    ) -> Self {
        Self {
            world: registry.world::<WorldRef>().clone(),
            computer_rows,
            hardware_rows,
            router_rows,
            users: registry.get(),
            lan: registry.get(),
            routers: registry.get(),
            settings: registry.get(),
            topology: registry.get(),
        }
    }
}
