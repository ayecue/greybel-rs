//! Router port forwards and firewall rules.

use std::rc::Rc;

use greyhack_behaviour::FirewallRule;

use crate::storage::*;
use crate::systems::RouterConfig as RouterConfigService;
use crate::*;

impl RouterConfigService {
    /// Returns `router` forwards in creation order.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn port_forwards(&self, router: RouterId) -> Result<Vec<PortForward>> {
        self.router_rows.port_forwards(router).await
    }

    /// Returns router public port `number` and its device forward.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn port_forward(&self, router: RouterId, number: u16) -> Result<Option<PortForward>> {
        self.router_rows.port_forward(router, number).await
    }

    /// Forwards router public `number` to computer `port`. Returns whether both ports exist and the
    /// public number was available.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn forward_port(
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

    /// Returns `router` firewall rules in insertion order.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn firewall_rules(&self, router: RouterId) -> Result<Vec<FirewallRule>> {
        self.router_rows.firewall_rules(router).await
    }

    /// Appends `rule` to `router` firewall rules.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn add_firewall_rule(&self, router: RouterId, rule: &FirewallRule) -> Result<()> {
        self.router_rows.insert_firewall_rule(router, rule).await
    }

    /// Deletes all `router` firewall rules.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn clear_firewall(&self, router: RouterId) -> Result<()> {
        self.router_rows.clear_firewall(router).await
    }
}

/// Router port-forward and firewall configuration.
pub struct RouterConfig {
    pub(crate) router_rows: Rc<RouterRepository>,
}

impl RouterConfig {
    pub(crate) fn new(router_rows: Rc<RouterRepository>) -> Self {
        Self { router_rows }
    }
}
