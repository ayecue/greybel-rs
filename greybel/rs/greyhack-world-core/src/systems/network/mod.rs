//! Networks: topology and reachability, connecting to services, LAN tables, routers and their configuration,
//! settings, and domain resolution.

use std::rc::Rc;

use greyhack_behaviour::FirewallRule;
use greyhack_world_modules::Dep;

use crate::storage::*;
use crate::systems::*;
use crate::*;

pub mod connect;
pub mod dns;
pub mod gateway;
pub mod generation;
pub mod lan;
pub mod reach;
pub mod router_config;
pub mod routers;
pub mod settings;
pub mod topology;

/// Network services.
pub struct NetworkService {
    pub(crate) router_rows: Rc<RouterRepository>,
    pub topology: Dep<NetworkTopology>,
    pub routers: Dep<RouterService>,
    pub lan: Dep<NetworkLan>,
    pub reach: Dep<NetworkReach>,
    pub connect: Dep<NetworkConnect>,
    pub dns: Dep<Dns>,
    pub router_config: Dep<RouterConfig>,
    pub settings: Dep<NetworkSettings>,
    pub net_cards: Dep<NetCardService>,
}

impl NetworkService {
    pub(crate) fn new(
        registry: &Rc<greyhack_world_modules::Registry>,
        router_rows: Rc<RouterRepository>,
    ) -> Self {
        Self {
            router_rows,
            topology: registry.get(),
            routers: registry.get(),
            lan: registry.get(),
            reach: registry.get(),
            connect: registry.get(),
            dns: registry.get(),
            router_config: registry.get(),
            settings: registry.get(),
            net_cards: registry.get(),
        }
    }
}

impl NetworkService {
    /// Removes every port-forwarding rule from `router`.
    pub async fn clear_port_forwards(&self, router: RouterId) -> Result<()> {
        self.router_rows.clear_port_forwards(router).await
    }

    /// Returns network `id`, if it exists.
    pub async fn network(&self, id: NetworkId) -> Result<Option<Network>> {
        self.router_rows.network(id).await
    }

    /// Sets the speed tier of `network`.
    pub async fn set_net_speed(&self, network: NetworkId, net_speed: u8) -> Result<()> {
        self.router_rows.set_net_speed(network, net_speed).await
    }

    /// Sets whether `network` is disabled.
    pub async fn disable_network(&self, network: NetworkId, disabled: bool) -> Result<()> {
        self.router_rows.disable_network(network, disabled).await
    }

    /// Sets the mission-contact address of `network`.
    pub async fn set_mission_contact(&self, network: NetworkId, contact: &str) -> Result<()> {
        self.router_rows.set_mission_contact(network, contact).await
    }

    /// Returns the mission-contact address of `network`, if any.
    pub async fn mission_contact(&self, network: NetworkId) -> Result<Option<String>> {
        self.router_rows.mission_contact(network).await
    }

    /// Stores `draft` and returns its network ID.
    pub async fn insert_network(&self, draft: &NetworkDraft) -> Result<NetworkId> {
        self.router_rows.insert_network(draft).await
    }

    /// Removes network `id` and its dependent rows.
    pub async fn remove_network(&self, id: NetworkId) -> Result<()> {
        self.router_rows.remove_network(id).await
    }

    /// Returns router `id`, if it exists.
    pub async fn router(&self, id: RouterId) -> Result<Option<Router>> {
        self.router_rows.router(id).await
    }

    /// Returns the router with `public_ip`, if any.
    pub async fn router_at(&self, public_ip: &str) -> Result<Option<Router>> {
        self.router_rows.router_at(public_ip).await
    }

    /// Returns the routers belonging to `network`.
    pub async fn routers_of(&self, network: NetworkId) -> Result<Vec<Router>> {
        self.router_rows.routers_of(network).await
    }

    /// Writes the mutable fields of `router`.
    pub async fn save_router(&self, router: &Router) -> Result<()> {
        self.routers.save_router(router).await
    }

    /// Stores `draft` in `network`, resolving libraries with `library_id`, and returns its router ID.
    pub async fn insert_router_with<L, S>(
        &self,
        draft: &RouterDraft<L, S>,
        network: NetworkId,
        place: Option<&LanPlace>,
        library_id: impl AsyncFn(&L) -> Result<crate::LibraryId>,
    ) -> Result<RouterId> {
        self.routers
            .insert_router_with(draft, network, place, library_id)
            .await
    }

    /// Adds a forwarding rule to `router` and reports whether it was inserted.
    pub async fn insert_port_forward(
        &self,
        router: RouterId,
        number: u16,
        computer: ComputerId,
        port: u16,
    ) -> Result<bool> {
        self.router_rows
            .insert_port_forward(router, number, computer, port)
            .await
    }

    /// Returns the firewall rules of `router`.
    pub async fn firewall_rules(&self, router: RouterId) -> Result<Vec<FirewallRule>> {
        self.router_rows.firewall_rules(router).await
    }

    /// Adds `rule` to the firewall of `router`.
    pub async fn insert_firewall_rule(&self, router: RouterId, rule: &FirewallRule) -> Result<()> {
        self.router_rows.insert_firewall_rule(router, rule).await
    }

    /// Removes every firewall rule from `router`.
    pub async fn clear_firewall(&self, router: RouterId) -> Result<()> {
        self.router_rows.clear_firewall(router).await
    }
}
