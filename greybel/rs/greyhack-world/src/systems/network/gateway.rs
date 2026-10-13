//! Port forwards and connections through a network's gateway.

use crate::{ComputerId, NetworkId, Result, RouterId, World};

impl World {
    /// Returns the number of forwards targeting a computer.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn port_forwards_to(&self, computer: ComputerId) -> Result<usize> {
        let Some(computer) = self.db.computer(computer).await? else {
            return Ok(0);
        };
        let mut count = 0;

        for router in self.db.routers_of(computer.network).await? {
            count += self
                .db
                .port_forwards(router.id)
                .await?
                .iter()
                .filter(|forward| forward.computer == computer.id)
                .count();
        }

        Ok(count)
    }

    /// Counts the ports a router's computer opens itself and the forwards targeting it
    /// (`NetSessionIntrinsics.get_num_portforward` on a router session).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn router_port_forwards(&self, router: RouterId) -> Result<usize> {
        let computer = router.computer();

        Ok(self.ports(computer).await?.len() + self.port_forwards_to(computer).await?)
    }

    /// Counts computers using `computer` as gateway (`NetworkLan.GetNumConnGateway`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn gateway_connections(&self, computer: ComputerId) -> Result<usize> {
        let Some(computer) = self.db.computer(computer).await? else {
            return Ok(0);
        };

        self.connections_to(computer.network, &computer.local_ip)
            .await
    }

    /// Counts computers using `router` as gateway (`NetworkLan.GetNumConnGateway`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn router_gateway_connections(&self, router: RouterId) -> Result<usize> {
        let Some(router) = self.db.router(router).await? else {
            return Ok(0);
        };

        self.connections_to(router.network, &router.local_ip).await
    }

    /// Counts computers in `network` using `gateway`.
    async fn connections_to(&self, network: NetworkId, gateway: &str) -> Result<usize> {
        Ok(self
            .lan(network)
            .await?
            .map_or(0, |lan| lan.computer_addresses(gateway, false, false).len()))
    }
}
