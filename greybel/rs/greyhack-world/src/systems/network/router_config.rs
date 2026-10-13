//! Router port forwards, firewall rules, and reverse-shell registrations.

use greyhack_behaviour::FirewallRule;

use crate::model::{ComputerId, PortForward, ReverseShell, RouterId};
use crate::{Result, World};

impl World {
    /// Returns `router` forwards in creation order.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn port_forwards(&self, router: RouterId) -> Result<Vec<PortForward>> {
        self.db.port_forwards(router).await
    }

    /// Returns router public port `number` and its device forward.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn port_forward(&self, router: RouterId, number: u16) -> Result<Option<PortForward>> {
        self.db.port_forward(router, number).await
    }

    /// Forwards router public `number` to computer `port`. Returns whether both ports exist and the
    /// public number was available.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn forward_port(
        &mut self,
        router: RouterId,
        number: u16,
        computer: ComputerId,
        port: u16,
    ) -> Result<bool> {
        self.db
            .insert_port_forward(router, number, computer, port)
            .await
    }

    /// Returns `router` firewall rules in insertion order.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn firewall_rules(&self, router: RouterId) -> Result<Vec<FirewallRule>> {
        self.db.firewall_rules(router).await
    }

    /// Appends `rule` to `router` firewall rules.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn add_firewall_rule(&mut self, router: RouterId, rule: &FirewallRule) -> Result<()> {
        self.db.insert_firewall_rule(router, rule).await
    }

    /// Deletes all `router` firewall rules.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn clear_firewall(&mut self, router: RouterId) -> Result<()> {
        self.db.clear_firewall(router).await
    }

    /// Returns reverse shells registered on `router`, oldest first.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn reverse_shells(&self, router: RouterId) -> Result<Vec<ReverseShell>> {
        self.db.reverse_shells(router).await
    }

    /// Registers `shell` on `router`; ending its client process removes it.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn add_reverse_shell(
        &mut self,
        router: RouterId,
        shell: &ReverseShell,
    ) -> Result<()> {
        self.db.insert_reverse_shell(router, shell).await
    }
}
