//! Placing the mission of a network in its LAN (`NetworkLan.CreateMission`, `NetworkLan.ConfigureChainMission`,
//! `NetworkLan.EnsureMissionComputers`).

use greyhack_behaviour::{DeviceKind, ServiceId, same_subnet};
use greyhack_world_missions::{ItemKind, MissionPlacement, assign_computers};

use super::Planner;
use super::addresses::format_address;
use super::post_generation::rule;
use crate::ForwardLayout;
use crate::generation::lan::{ComputerPlan, ServicePlan};

impl Planner<'_> {
    /// `NetworkLan.GetAllComputers`: the standard computers of `device`, then those of each child device.
    pub(crate) fn computers_in_order(&self, device: usize, addresses: &mut Vec<String>) {
        addresses.extend(
            self.devices[device]
                .computers
                .iter()
                .filter(|computer| computer.kind == DeviceKind::Computer)
                .map(|computer| computer.local_ip.clone()),
        );

        for child in self.children(device) {
            self.computers_in_order(child, addresses);
        }
    }

    /// `NetworkLan.Ping` from the main router to a standard computer: a computer behind a switch or a private gateway
    /// is only reachable from its own subnet.
    pub(crate) fn reachable_from_router(&self, computer: &str) -> bool {
        let router = &self.devices[0].local_ip;
        let Some(gateway) = self
            .devices
            .iter()
            .flat_map(|device| &device.computers)
            .find(|candidate| candidate.local_ip == computer)
            .map(|candidate| candidate.gateway.as_str())
        else {
            return false;
        };

        if let Some(device) = self
            .devices
            .iter()
            .find(|device| !device.local_ip.is_empty() && device.local_ip == gateway)
        {
            return !device.is_switch() || same_subnet(router, computer);
        }

        let behind_computer = self
            .devices
            .iter()
            .flat_map(|device| &device.computers)
            .any(|candidate| !gateway.is_empty() && candidate.local_ip == gateway);

        behind_computer && same_subnet(router, computer)
    }

    /// `NetworkLan.EnsureMissionComputers`: the standard computers the mission can use, with computers added to the main
    /// router when the network has fewer than `count`. With `reachable_only`, only those the main router can reach
    /// (and the added ones) are listed.
    pub(crate) fn mission_computers(&mut self, count: usize, reachable_only: bool) -> Vec<String> {
        let mut available = Vec::new();

        self.computers_in_order(0, &mut available);

        if reachable_only {
            available.retain(|computer| self.reachable_from_router(computer));
        }

        if count <= available.len() {
            return available;
        }

        let router_ip = self.devices[0].local_ip.clone();
        let mut extra = Vec::new();

        for _ in available.len()..count {
            let local_ip = format_address(self.next_ip(&router_ip));

            self.attach(
                0,
                ComputerPlan {
                    kind: DeviceKind::Computer,
                    local_ip: local_ip.clone(),
                    gateway: String::new(),
                    services: Vec::new(),
                },
            );
            extra.push(local_ip);
        }

        if reachable_only {
            available.extend(extra);
        } else {
            available.clear();
            self.computers_in_order(0, &mut available);
        }

        available
    }

    /// `NetworkLan.CreateMission`: picks a computer for each computer of the mission (`ConfigureChainMission` for the
    /// networks of a chain), adding computers to the main router when the network has too few, and for a chain adds
    /// the services and firewall rules the mission needs.
    pub(super) fn create_mission(&mut self) {
        let Some(request) = self.mission.take() else {
            return;
        };
        let count = request.part.computers.len();
        let available = self.mission_computers(count, request.needs_reachable_computers());

        if available.len() < count {
            return;
        }

        let assignments = assign_computers(&request, available, |length| {
            self.next(
                0,
                u32::try_from(length).expect("value fits the target integer type"),
            ) as usize
        });
        let mut websites = Vec::new();

        if request.is_chain() {
            let forwards = request.kind().is_procedural() || request.needs_reachable_computers();

            for assignment in &assignments {
                for item in &request.part.computers[assignment.computer].items {
                    if let ItemKind::Service { service, .. } = &item.kind
                        && let Some(service) = ServiceId::from_code(i64::from(*service))
                        && self.add_mission_service(&assignment.local_ip, service, forwards)
                        && service == ServiceId::Http
                    {
                        websites.push((assignment.local_ip.clone(), item.id.clone()));
                    }
                }

                if request.needs_reachable_computers() {
                    self.swap_to_default_port(&assignment.local_ip, &request, assignment.computer);
                }
            }
        }

        let placement = MissionPlacement {
            local_ips: assignments
                .iter()
                .map(|assignment| assignment.local_ip.clone())
                .collect(),
            assignments,
            websites,
            request,
        };

        self.devices[0].firewall.extend(placement.firewall_rules());
        self.placement = Some(placement);
    }

    /// The service part of `ConfigureChainMission`: a computer without `service` gets it with a main-router forward
    /// (`PreAddServicio`); one with it gets a forward when it has none (`AddPortForwardExistingService`) in a
    /// procedural or zero-day mission. Returns whether the computer got the service.
    pub(crate) fn add_mission_service(
        &mut self,
        local_ip: &str,
        service: ServiceId,
        forward_existing: bool,
    ) -> bool {
        let Some((device, at)) = self.devices.iter().enumerate().find_map(|(device, plan)| {
            plan.computers
                .iter()
                .position(|computer| computer.local_ip == local_ip)
                .map(|at| (device, at))
        }) else {
            return false;
        };
        let current_port = self.devices[device].computers[at]
            .services
            .iter()
            .find(|plan| plan.service == service)
            .map(|plan| plan.port);

        let port = match current_port {
            Some(port) => {
                if !forward_existing
                    || self
                        .table
                        .iter()
                        .any(|forward| forward.lan_ip == local_ip && forward.port == port)
                {
                    return false;
                }

                port
            }
            None => {
                let mut port = service.default_port();

                while self.devices[device].computers[at]
                    .services
                    .iter()
                    .any(|other| other.port == port)
                {
                    port += 1;
                }

                self.devices[device].computers[at]
                    .services
                    .push(ServicePlan {
                        service,
                        port,
                        closed: false,
                    });

                port
            }
        };
        let number = self.external_port(port, service);

        self.table.push(ForwardLayout {
            number,
            lan_ip: local_ip.to_owned(),
            port,
        });

        if !self.devices[device].is_hub() {
            self.router_forward_addresses.push(local_ip.to_owned());
        }

        current_port.is_none()
    }

    /// The zero-day part of `ConfigureChainMission`: each service of the mission on the computer is forwarded from the
    /// service's own port, taking that port from the forward that had it, and the firewalls above the computer allow it.
    pub(crate) fn swap_to_default_port(
        &mut self,
        local_ip: &str,
        request: &greyhack_world_missions::MissionRequest,
        computer: usize,
    ) {
        for item in &request.part.computers[computer].items {
            let ItemKind::Service { service, .. } = &item.kind else {
                continue;
            };
            let Some(service) = ServiceId::from_code(i64::from(*service)) else {
                continue;
            };
            let Some(internal) = self
                .devices
                .iter()
                .flat_map(|device| &device.computers)
                .find(|candidate| candidate.local_ip == local_ip)
                .and_then(|candidate| {
                    candidate
                        .services
                        .iter()
                        .find(|plan| plan.service == service)
                        .map(|plan| plan.port)
                })
            else {
                continue;
            };
            let Some(own) = self
                .table
                .iter()
                .position(|forward| forward.lan_ip == local_ip && forward.port == internal)
            else {
                continue;
            };
            let target = service.default_port();

            if let Some(other) = self
                .table
                .iter()
                .position(|forward| forward.number == target)
            {
                self.table[other].number = self.table[own].number;
            }

            self.table[own].number = target;
            self.allow_through_firewalls(local_ip);
        }
    }

    /// `AllowMainServicesFw(device, lanIP)`: allows traffic to `lan_ip` through every firewall above it.
    pub(super) fn allow_through_firewalls(&mut self, lan_ip: &str) {
        let mut device = self.holder_of(lan_ip);

        while let Some(current) = device {
            if !self.devices[current].firewall.is_empty() {
                self.devices[current].firewall.push(rule(
                    true,
                    None,
                    None,
                    Some(lan_ip.to_owned()),
                ));
            }

            device = self.devices[current].parent;
        }
    }
}
