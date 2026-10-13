//! LAN structure and queries matching `NetworkLan`. The main router is the root of a device tree;
//! LAN routers, switches, and hubs have attached computers and child devices.
//!
//! Tree parentage and gateways are independent. Computers under a hub can use their adopting router as
//! the gateway (`NetworkLan.ConfigPendingRouter`). Reachability follows gateways; firewall paths follow
//! the tree.

mod entry;
mod node;
mod node_id;
mod port;

use entry::Entry;
use node::rank;
pub use node::{LanNode, is_device};
pub use node_id::LanNodeId;
pub use port::LanPort;

use crate::{DeviceKind, is_lan_ip, is_valid_ip, same_subnet};

/// LAN traversal errors after reaching a computer (`PlayerUtils.FirewallPingStatus`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanFailure {
    /// The destination does not respond to the port-table entry ping.
    Unreachable,
    /// A firewall along the path blocks the connection.
    Firewall,
}

/// LAN tree (`NetworkLan`) rooted at the main router.
#[derive(Debug, Clone, PartialEq)]
pub struct Lan<K> {
    entries: Vec<Entry<K>>,
}

impl<K> Lan<K> {
    /// Creates a LAN containing only its main router.
    #[must_use]
    pub fn new(router: LanNode<K>) -> Self {
        Self {
            entries: vec![Entry {
                node: router,
                parent: None,
                children: Vec::new(),
            }],
        }
    }

    /// Returns the main router ID.
    #[must_use]
    pub const fn root(&self) -> LanNodeId {
        LanNodeId(0)
    }

    /// Inserts `node` under `parent` after existing children of the same rank.
    pub fn add(&mut self, parent: LanNodeId, node: LanNode<K>) -> LanNodeId {
        let id = LanNodeId(self.entries.len());
        let node_rank = rank(node.kind);

        self.entries.push(Entry {
            node,
            parent: Some(parent),
            children: Vec::new(),
        });

        let at = self.entries[parent.0]
            .children
            .iter()
            .position(|child| rank(self.entries[child.0].node.kind) > node_rank)
            .unwrap_or(self.entries[parent.0].children.len());

        self.entries[parent.0].children.insert(at, id);

        id
    }

    #[must_use]
    pub fn node(&self, id: LanNodeId) -> &LanNode<K> {
        &self.entries[id.0].node
    }

    pub fn node_mut(&mut self, id: LanNodeId) -> &mut LanNode<K> {
        &mut self.entries[id.0].node
    }

    /// Returns the parent device of `id`, or `None` for the main router.
    #[must_use]
    pub fn parent(&self, id: LanNodeId) -> Option<LanNodeId> {
        self.entries[id.0].parent
    }

    /// Returns children of `id`: computers, then hubs and switches, then routers.
    #[must_use]
    pub fn children(&self, id: LanNodeId) -> &[LanNodeId] {
        &self.entries[id.0].children
    }

    /// Iterates over all nodes, starting with the main router.
    pub fn nodes(&self) -> impl Iterator<Item = (LanNodeId, &LanNode<K>)> {
        self.entries
            .iter()
            .enumerate()
            .map(|(index, entry)| (LanNodeId(index), &entry.node))
    }

    /// Returns the game's search order: each device, its computers, its hubs and switches, then its
    /// routers and their descendants.
    #[must_use]
    pub fn search_order(&self) -> Vec<LanNodeId> {
        let mut order = Vec::with_capacity(self.entries.len());
        let mut stack = vec![self.root()];

        while let Some(id) = stack.pop() {
            order.push(id);
            stack.extend(self.children(id).iter().rev());
        }

        order
    }

    fn devices(&self, id: LanNodeId) -> impl Iterator<Item = LanNodeId> + '_ {
        self.children(id)
            .iter()
            .copied()
            .filter(|child| self.node(*child).is_device())
    }

    /// Finds the first node at `address` in [`Lan::search_order`] (`NetworkLan.FindDevice`).
    #[must_use]
    pub fn find(&self, address: &str) -> Option<LanNodeId> {
        if address.is_empty() {
            return None;
        }

        self.search_order()
            .into_iter()
            .find(|id| self.node(*id).local_ip == address)
    }

    /// Returns the gateway of node `id`: a computer's gateway or a device's own address (`HelperPing.GetGateway`).
    #[must_use]
    pub fn gateway(&self, id: LanNodeId) -> &str {
        let node = self.node(id);

        if node.is_device() {
            &node.local_ip
        } else {
            &node.gateway
        }
    }

    /// Returns a node's parent device, or the node itself when it is a device (`GetDevicesPath` start).
    fn holder(&self, id: LanNodeId) -> LanNodeId {
        if self.node(id).is_device() {
            id
        } else {
            self.parent(id).unwrap_or(id)
        }
    }

    /// Checks whether `address` responds to `from` (`NetworkLan.Ping`). Public addresses always respond;
    /// callers must separately check whether a network exists behind them.
    #[must_use]
    pub fn ping(&self, from: &str, address: &str) -> bool {
        if !is_valid_ip(address) || !is_valid_ip(from) {
            return false;
        }

        if from == address || !is_lan_ip(address) {
            return true;
        }

        let Some(mut target) = self.find(address) else {
            return false;
        };
        let mut computer = false;

        if !self.node(target).is_device() {
            if self.node(target).hidden {
                return false;
            }

            computer = true;

            let Some(gateway) = self.find(self.gateway(target)) else {
                return false;
            };

            target = gateway;
        }

        if self.find(from).is_none() {
            return false;
        }

        let node = self.node(target);

        if !node.is_device() || (node.kind == DeviceKind::Switch && computer) {
            return same_subnet(from, address);
        }

        true
    }

    /// Lists computers using `gateway` in search order (`NetworkLan.GetComputersIPs`). When
    /// `same_subnet_devices` is set, also lists other devices in `gateway`'s subnet.
    #[must_use]
    pub fn computer_addresses(
        &self,
        gateway: &str,
        same_subnet_devices: bool,
        without_hidden: bool,
    ) -> Vec<String> {
        let mut addresses = Vec::new();

        self.collect_computers(
            self.root(),
            gateway,
            same_subnet_devices,
            without_hidden,
            &mut addresses,
        );

        addresses
    }

    /// Collects child computers, then this device, then descendant devices in `GetComputersIPs` order.
    fn collect_computers(
        &self,
        device: LanNodeId,
        gateway: &str,
        same_subnet_devices: bool,
        without_hidden: bool,
        addresses: &mut Vec<String>,
    ) {
        for child in self.children(device) {
            let node = self.node(*child);

            if !node.is_device() && node.gateway == gateway && !(without_hidden && node.hidden) {
                addresses.push(node.local_ip.clone());
            }
        }

        let own = &self.node(device).local_ip;

        if same_subnet_devices && own != gateway && same_subnet(own, gateway) {
            addresses.push(own.clone());
        }

        for child in self.devices(device).collect::<Vec<_>>() {
            self.collect_computers(
                child,
                gateway,
                same_subnet_devices,
                without_hidden,
                addresses,
            );
        }
    }

    /// Lists routers and switches that respond to `from` (`NetworkLan.GetRoutersIPs`).
    #[must_use]
    pub fn device_addresses(&self, from: &str) -> Vec<String> {
        self.search_order()
            .into_iter()
            .map(|id| self.node(id))
            .filter(|node| node.is_device() && self.ping(from, &node.local_ip))
            .map(|node| node.local_ip.clone())
            .collect()
    }

    /// Lists the LAN exposed by `router_ip` (`Router.GetComputersIPs`, `devices_lan_ip`): computers
    /// using it as gateway, followed by all network devices.
    #[must_use]
    pub fn router_lan(&self, router_ip: &str, without_hidden: bool) -> Vec<String> {
        let mut addresses = self.computer_addresses(router_ip, false, without_hidden);

        addresses.extend(self.device_addresses(router_ip));

        addresses
    }

    /// Selects the port table for `destination` (`NetworkLan.ToLocalIPs`): the main router for public
    /// addresses and its own LAN address, the target device for device addresses, or none for computers.
    #[must_use]
    pub fn port_table(&self, destination: &str) -> Option<LanNodeId> {
        if !is_lan_ip(destination) || destination == self.node(self.root()).local_ip {
            return Some(self.root());
        }

        self.find(destination)
            .filter(|id| self.node(*id).is_device())
    }

    /// Resolves LAN addresses for `source` to `destination:port` (`NetworkLan.ToLocalIPs`). Public sources
    /// start at the main router, and forwarded router ports resolve to their target LAN address.
    #[must_use]
    pub fn local_addresses(&self, source: &str, destination: &str, port: u16) -> (String, String) {
        let source = if is_lan_ip(source) {
            source.to_owned()
        } else {
            self.node(self.root()).local_ip.clone()
        };
        let destination = self
            .port_table(destination)
            .and_then(|router| {
                self.node(router)
                    .ports
                    .iter()
                    .find(|table_port| table_port.number == port)
            })
            .map_or_else(|| destination.to_owned(), |port| port.lan_ip.clone());

        (source, destination)
    }

    /// Returns the device path from `origin` to `destination` (`NetworkLan.GetDevicesPath`), starting
    /// at their closest common device. Returns an empty path when `destination` is outside the LAN.
    #[must_use]
    pub fn path(&self, origin: LanNodeId, destination: &str) -> Vec<LanNodeId> {
        let mut path = Vec::new();
        let mut device = self.holder(origin);

        if self.search_path(device, destination, None, &mut path) {
            return path;
        }

        while let Some(parent) = self.parent(device) {
            if self.search_path(parent, destination, Some(device), &mut path) {
                break;
            }

            device = parent;
        }

        path
    }

    fn search_path(
        &self,
        device: LanNodeId,
        destination: &str,
        skip: Option<LanNodeId>,
        path: &mut Vec<LanNodeId>,
    ) -> bool {
        let has_destination = self.node(device).local_ip == destination
            || self.children(device).iter().any(|child| {
                let child = self.node(*child);

                !child.is_device() && child.local_ip == destination
            });

        if has_destination {
            path.push(device);

            return true;
        }

        for child in self.devices(device).collect::<Vec<_>>() {
            if Some(child) != skip && self.search_path(child, destination, None, path) {
                path.push(device);

                return true;
            }
        }

        false
    }

    /// Returns why `source` cannot reach `destination:port` after router port-table resolution
    /// (`PlayerUtils.FirewallPingStatus`), or `None` when it can.
    #[must_use]
    pub fn firewall_ping_status(
        &self,
        source: &str,
        destination: &str,
        port: u16,
    ) -> Option<LanFailure> {
        let (entry, destination) = self.local_addresses(source, destination, port);

        if !self.ping(&entry, &destination) {
            return Some(LanFailure::Unreachable);
        }

        if self.blocks(source, &entry, port, &destination) {
            return Some(LanFailure::Firewall);
        }

        None
    }

    /// Checks firewalls between `entry` and `destination:port` for `source`, which is public for external
    /// connections (`NetworkLan.IsFirewallBlocking`). A device blocks the connection when a DENY rule
    /// matches and no ALLOW rule matches.
    #[must_use]
    pub fn blocks(&self, source: &str, entry: &str, port: u16, destination: &str) -> bool {
        let Some(entry) = self.find(entry) else {
            return false;
        };

        self.path(entry, destination).into_iter().any(|device| {
            let rules = &self.node(device).firewall;
            let matches = |allow: bool| {
                rules
                    .iter()
                    .any(|rule| rule.allow == allow && rule.matches(source, port, destination))
            };

            matches(false) && !matches(true)
        })
    }

    /// Returns one address past the highest address used by a computer of `gateway` or a device in
    /// `gateway`'s subnet (`NetworkLan.GetNextIP`).
    #[must_use]
    pub fn next_address(&self, gateway: &str) -> String {
        let mut highest = gateway.to_owned();

        for address in self.computer_addresses(gateway, true, false) {
            if same_subnet(&highest, &address) && last_part(&address) > last_part(&highest) {
                highest = address;
            }
        }

        with_last_part(&highest, last_part(&highest) + 1)
    }
}

/// Parses the final component of `address`, returning 0 when absent (`IP.NextIP` does not validate it).
fn last_part(address: &str) -> u32 {
    address
        .rsplit('.')
        .next()
        .and_then(|part| part.parse().ok())
        .unwrap_or(0)
}

/// Replaces the final component of `address` with `last` (`IP.SetLastRange`), allowing values above 255.
fn with_last_part(address: &str, last: u32) -> String {
    match address.rfind('.') {
        Some(at) => format!("{}.{last}", &address[..at]),
        None => address.to_owned(),
    }
}
