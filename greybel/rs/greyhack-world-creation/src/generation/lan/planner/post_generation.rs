//! The planning that follows generation (`PostGeneration`): adoption, main services, and allowed addresses.

use greyhack_behaviour::{DeviceKind, FirewallRule, ROUTER_PORT, same_subnet};

use super::Planner;
use super::services::repeatable;

pub(super) fn rule(
    allow: bool,
    port: Option<u16>,
    source: Option<String>,
    destination: Option<String>,
) -> FirewallRule {
    FirewallRule {
        allow,
        port,
        source,
        destination,
    }
}

impl Planner<'_> {
    /// `NetworkLan.PostGeneration`: adds a router for waiting hub computers, a main-services
    /// router, a camera if smart appliances exist without one, and firewall ALLOW rules.
    pub(super) fn post_generation(&mut self) {
        if !self.pending_hub.is_empty() && !self.adopt_somewhere(0) {
            let children = self.children(0);

            if !children.is_empty() {
                let parent = children[self.next(
                    0,
                    u32::try_from(children.len()).expect("value fits the target integer type"),
                ) as usize];
                let router = self.adoption_router(parent);

                self.devices[parent].routers.push(router);
            }
        }

        self.main_services();

        if self.device_draw.cameras() == 0 && self.has_smart_appliance() {
            let address = self.next_ip(&self.devices[0].local_ip.clone());

            self.camera(0, address);
        }

        self.allow_addresses();
        self.create_mission();
        self.allow_main_services();
        self.config_main_firewall();
    }

    pub(super) fn children(&self, device: usize) -> Vec<usize> {
        let device = &self.devices[device];

        device.hubs.iter().chain(&device.routers).copied().collect()
    }

    /// `CrearGateway`: attaches a router for waiting computers to the first device selected by a
    /// one-in-four draw.
    pub(crate) fn adopt_somewhere(&mut self, device: usize) -> bool {
        if self.next(0, 4) == 0 {
            let router = self.adoption_router(device);

            self.devices[device].routers.push(router);

            return true;
        }

        for child in self.children(device) {
            if self.adopt_somewhere(child) {
                return true;
            }
        }

        false
    }

    /// `ConfigureMainServices`: assigns unassigned public services to the first computer of an
    /// additional router.
    pub(crate) fn main_services(&mut self) {
        self.public
            .retain(|entry| !repeatable(entry.service, self.kind));

        let children = self.children(0);

        if self.public.is_empty() || children.is_empty() {
            return;
        }

        let parent = children[self.next(
            0,
            u32::try_from(children.len()).expect("value fits the target integer type"),
        ) as usize];
        let router = self.adoption_router(parent);
        let computers = self.next(1, 5);

        self.computers(router, computers, true, None);
        self.devices[parent].routers.push(router);
    }

    /// `ConfigMainFirewall`: a main router that has rules (only a mission gives it any) allows each forwarded port.
    pub(crate) fn config_main_firewall(&mut self) {
        if self.devices[0].firewall.is_empty() {
            return;
        }

        let router_ip = self.devices[0].local_ip.clone();
        let forwards: Vec<(u16, String)> = self
            .table
            .iter()
            .filter(|forward| forward.lan_ip != router_ip)
            .map(|forward| (forward.number, forward.lan_ip.clone()))
            .collect();

        for (number, lan_ip) in forwards {
            self.devices[0]
                .firewall
                .push(rule(true, Some(number), None, Some(lan_ip)));
        }
    }

    /// `AllowMainServicesFw`: adds ALLOW rules to every firewall above a forwarded computer.
    pub(crate) fn allow_main_services(&mut self) {
        if self.deny_devices.is_empty() {
            return;
        }

        let targets: Vec<String> = self
            .table
            .iter()
            .filter(|forward| forward.number != ROUTER_PORT)
            .map(|forward| forward.lan_ip.clone())
            .collect();

        for lan_ip in targets {
            self.allow_through_firewalls(&lan_ip);
        }
    }

    /// Returns the device containing the computer at `lan_ip`, or its parent device
    /// (`FindDevice(..).GetParent()`).
    pub(super) fn holder_of(&self, lan_ip: &str) -> Option<usize> {
        for (index, device) in self.devices.iter().enumerate() {
            if device.local_ip == lan_ip {
                return device.parent;
            }

            if device
                .computers
                .iter()
                .any(|computer| computer.local_ip == lan_ip)
            {
                return Some(index);
            }
        }

        None
    }

    /// `AllowAddressFw`: each denying firewall allows one to four external computers, or denies
    /// nothing if none exist.
    pub(crate) fn allow_addresses(&mut self) {
        for device in self.deny_devices.clone() {
            let count = self.next(1, 5);
            let sources = self.allowed_computers(device, count);

            if sources.is_empty() {
                self.devices[device].firewall.remove(0);
            }

            for source in sources {
                self.devices[device]
                    .firewall
                    .push(rule(true, None, Some(source), None));
            }
        }
    }

    /// `GetPropFwComputers`: selects up to `count` non-camera computers outside the `device` subnet
    /// from unfirewalled neighboring and ancestor devices.
    pub(crate) fn allowed_computers(&mut self, device: usize, count: u32) -> Vec<String> {
        let own_subnet = self.devices[device].local_ip.clone();
        let mut candidates: Vec<(String, DeviceKind)> = Vec::new();
        let mut child = own_subnet.clone();
        let mut current = self.devices[device].parent;

        while let Some(holder) = current {
            for peer in self.children(holder) {
                let peer = &self.devices[peer];

                if peer.firewall.is_empty() && peer.local_ip != child {
                    candidates.extend(
                        peer.computers
                            .iter()
                            .map(|computer| (computer.local_ip.clone(), computer.kind)),
                    );
                }
            }

            if !self.devices[holder].firewall.is_empty() {
                break;
            }

            candidates.extend(
                self.devices[holder]
                    .computers
                    .iter()
                    .map(|computer| (computer.local_ip.clone(), computer.kind)),
            );
            child.clone_from(&self.devices[holder].local_ip);
            current = self.devices[holder].parent;
        }

        candidates.retain(|(address, kind)| {
            !same_subnet(address, &own_subnet) && *kind != DeviceKind::Camera
        });

        let mut sample = Vec::new();

        for _ in 0..count {
            if candidates.is_empty() {
                break;
            }

            let index = self.next(
                0,
                u32::try_from(candidates.len()).expect("value fits the target integer type"),
            ) as usize;

            sample.push(candidates.remove(index).0);
        }

        sample
    }
}
