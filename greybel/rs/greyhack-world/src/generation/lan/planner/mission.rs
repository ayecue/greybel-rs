//! Placing the mission of a network in its LAN (`NetworkLan.CreateMission`).

use greyhack_behaviour::{DeviceKind, ServiceId};

use super::Planner;
use super::addresses::format_address;
use crate::ForwardLayout;
use crate::generation::lan::{ComputerPlan, ServicePlan};
use crate::generation::missions::{Assignment, ItemKind, MissionPlacement};

impl Planner<'_> {
    /// `NetworkLan.GetAllComputers`: the standard computers of `device`, then those of each child device.
    fn computers_in_order(&self, device: usize, addresses: &mut Vec<String>) {
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

    /// `NetworkLan.CreateMission` (`ConfigureChainMission` for a procedural mission): picks a computer for each
    /// computer of the mission, adding computers to the main router when the network has too few
    /// (`EnsureMissionComputers`), and adds the services the mission needs.
    pub(super) fn create_mission(&mut self) {
        let Some(request) = self.mission.take() else {
            return;
        };
        let count = request.part.computers.len();
        let mut available = Vec::new();

        self.computers_in_order(0, &mut available);

        if available.len() < count {
            let router_ip = self.devices[0].local_ip.clone();

            for _ in available.len()..count {
                let local_ip = format_address(self.next_ip(&router_ip));

                self.attach(
                    0,
                    ComputerPlan {
                        kind: DeviceKind::Computer,
                        local_ip,
                        gateway: String::new(),
                        services: Vec::new(),
                    },
                );
            }

            available.clear();
            self.computers_in_order(0, &mut available);
        }

        let mut random = mono_random::Random::new(request.seed);
        let mut assignments: Vec<Assignment> = Vec::new();
        let mut placed_npcs: Vec<i32> = Vec::new();

        for (index, computer) in request.part.computers.iter().enumerate() {
            let at = if request.index == 0 {
                self.next(0, available.len() as u32) as usize
            } else {
                random.next_below(available.len())
            };
            let local_ip = available.remove(at);
            let npc = computer.clone_npc_pc_idx;
            let replacement = (npc >= 0).then(|| random.next_double());

            assignments.push(Assignment {
                local_ip,
                computer: index,
                replacement,
                npc_in_lan: npc >= 0 && placed_npcs.contains(&npc),
            });

            if npc >= 0 {
                placed_npcs.push(npc);
            }
        }

        let mut websites = Vec::new();

        for assignment in &assignments {
            for item in &request.part.computers[assignment.computer].items {
                if let ItemKind::Service { service, .. } = &item.kind
                    && let Some(service) = ServiceId::from_code(i64::from(*service))
                {
                    self.add_mission_service(&assignment.local_ip, service);

                    if service == ServiceId::Http && !item.id.is_empty() {
                        websites.push((assignment.local_ip.clone(), item.id.clone()));
                    }
                }
            }
        }

        self.placement = Some(MissionPlacement {
            local_ips: assignments
                .iter()
                .map(|assignment| assignment.local_ip.clone())
                .collect(),
            assignments,
            websites,
            request,
        });
    }

    /// The service part of `ConfigureChainMission`: a computer without `service` gets it with a main-router forward
    /// (`PreAddServicio`), one with it gets a forward when it has none (`AddPortForwardExistingService`).
    fn add_mission_service(&mut self, local_ip: &str, service: ServiceId) {
        let Some((device, at)) = self.devices.iter().enumerate().find_map(|(device, plan)| {
            plan.computers
                .iter()
                .position(|computer| computer.local_ip == local_ip)
                .map(|at| (device, at))
        }) else {
            return;
        };
        let current_port = self.devices[device].computers[at]
            .services
            .iter()
            .find(|plan| plan.service == service)
            .map(|plan| plan.port);

        let port = match current_port {
            Some(port) => {
                if self
                    .table
                    .iter()
                    .any(|forward| forward.lan_ip == local_ip && forward.port == port)
                {
                    return;
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
    }
}
