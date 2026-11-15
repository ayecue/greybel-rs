//! Database-backed `NetworkLan` tree queries.

use std::rc::Rc;

use greyhack_behaviour::{DeviceKind, Lan, LanNode, LanPort};
use greyhack_world_modules::Dep;

use crate::storage::{ComputerRepository, RouterRepository};
use crate::systems::NetworkLan as NetworkLanService;
use crate::{Computer, ComputerId, NetworkId, NpcLookup, Port, Result, Router, RouterId, WorldRef};

/// Router port-table entry (`Puerto`): external port `externalPort` forwards to `internalPort` on
/// the device at `lan_ip`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TablePort {
    pub number: u16,
    pub lan_ip: String,
    pub port: Port,
}

impl NetworkLanService {
    /// Returns `router`'s port table (`RouterConfigOS.puertos`): local ports followed by forwards
    /// in creation order.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn port_table(&self, router: RouterId) -> Result<Vec<TablePort>> {
        let Some(row) = self.computer_rows.computer(router.computer()).await? else {
            return Ok(Vec::new());
        };
        let own = self
            .computer_rows
            .ports(row.id)
            .await?
            .into_iter()
            .map(|port| TablePort {
                number: port.number,
                lan_ip: row.local_ip.clone(),
                port,
            });
        let forwards = self
            .router_rows
            .port_forwards(router)
            .await?
            .into_iter()
            .map(|forward| TablePort {
                number: forward.number,
                lan_ip: forward.lan_ip,
                port: forward.port,
            });

        Ok(own.chain(forwards).collect())
    }

    /// Returns `network`'s LAN, or `None` if deleted. Includes devices and computers, firewall
    /// rules only while enabled, and port tables with local ports before forwards.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn lan(&self, network: NetworkId) -> Result<Option<Lan<ComputerId>>> {
        let computers = self.computer_rows.computers_in(network).await?;
        let routers = self.router_rows.routers_of(network).await?;
        let Some(main) = computers.iter().find(|computer| computer.parent.is_none()) else {
            return Ok(None);
        };
        let mut lan = Lan::new(self.lan_node(main, &routers).await?);
        // Children retain insertion order (`computers_in` sorting), and every device precedes its
        // descendants.
        let mut stack = vec![(main.id, lan.root())];

        while let Some((device, node)) = stack.pop() {
            let mut children = Vec::new();

            for computer in computers
                .iter()
                .filter(|computer| computer.parent == Some(device))
            {
                let child = lan.add(node, self.lan_node(computer, &routers).await?);

                children.push((computer.id, child));
            }

            stack.extend(children.into_iter().rev());
        }

        Ok(Some(lan))
    }

    /// Returns the LAN containing `computer`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn lan_of(&self, computer: ComputerId) -> Result<Option<Lan<ComputerId>>> {
        match self.computer_rows.computer(computer).await? {
            Some(computer) => self.lan(computer.network).await,
            None => Ok(None),
        }
    }

    /// Returns whether `computer` is unreachable (`LanPcInfo.IsHidden`): permanently hidden, or an
    /// NPC smartphone whose owner is dead or not at work.
    async fn is_hidden(&self, computer: &Computer) -> Result<bool> {
        if computer.hidden {
            return Ok(true);
        }

        let Some(owner) = computer
            .npc_owner
            .filter(|_| computer.kind == DeviceKind::Smartphone)
        else {
            return Ok(false);
        };

        let state = self
            .npcs
            .npc_state(&self.world, NpcLookup::Person(owner))
            .await?;

        Ok(state.is_some_and(|npc| npc.dead || !npc.at_work))
    }

    async fn lan_node(
        &self,
        computer: &Computer,
        routers: &[Router],
    ) -> Result<LanNode<ComputerId>> {
        let mut node = LanNode {
            gateway: computer.gateway.clone(),
            hidden: self.is_hidden(computer).await?,
            ..LanNode::new(computer.id, computer.kind, &computer.local_ip)
        };

        if let Some(router) = routers
            .iter()
            .find(|router| router.id.computer() == computer.id)
        {
            if router.firewall_active {
                node.firewall = self.router_rows.firewall_rules(router.id).await?;
            }

            node.ports = self
                .port_table(router.id)
                .await?
                .into_iter()
                .map(|port| LanPort {
                    number: port.number,
                    lan_ip: port.lan_ip,
                })
                .collect();
        }

        Ok(node)
    }
}

/// Router LAN-tree rules.
pub struct NetworkLan {
    pub(crate) world: WorldRef,
    pub(crate) computer_rows: Rc<ComputerRepository>,
    pub(crate) router_rows: Rc<RouterRepository>,
    pub npcs: Dep<dyn crate::NpcState>,
}

impl NetworkLan {
    pub(crate) fn new(
        registry: &Rc<greyhack_world_modules::Registry>,
        computer_rows: Rc<ComputerRepository>,
        router_rows: Rc<RouterRepository>,
    ) -> Self {
        Self {
            world: registry.world::<WorldRef>().clone(),
            npcs: registry.get(),
            computer_rows,
            router_rows,
        }
    }
}
