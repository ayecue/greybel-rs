//! Planning the computers of a device: servers, cameras, appliances, and their addresses and ports.

use greyhack_behaviour::{DeviceKind, NetworkKind, ServiceId, same_subnet};

use super::Planner;
use super::addresses::{format_address, next_address, parse};
use super::services::{main_service, repeatable};
use crate::ForwardLayout;
use crate::generation::lan::{ComputerPlan, Pool, PoolService, ServicePlan};

impl Planner<'_> {
    /// `PreCrearSubnet`: creates `count` computers for `device`. At most two are servers unless
    /// `servers` assigns every remaining public service to the first computer.
    pub(super) fn computers(
        &mut self,
        device: usize,
        count: u32,
        servers: bool,
        start: Option<[u32; 4]>,
    ) {
        let mut pool = Pool::Public;
        let isolated =
            !servers && self.devices[device].is_hub() && count > 1 && self.next(0, 4) < 1;

        if isolated {
            let address = next_address(self.new_subnet_base(), 1);
            let gateway = format_address(address);

            self.devices[device].private_gateway = gateway.clone();
            self.devices[device].computers.push(ComputerPlan {
                kind: DeviceKind::Computer,
                local_ip: gateway,
                gateway: greyhack_behaviour::NO_ADDRESS.to_owned(),
                services: Vec::new(),
            });
            self.cursor = address;

            let index = self.devices[device].computers.len() - 1;

            self.pending_gateways.push(((device, index), count));
        }

        if servers {
            self.public
                .retain(|entry| !repeatable(entry.service, self.kind));
        }

        let mut server_count = 0;
        let mut address = start.unwrap_or(self.cursor);

        for _ in 0..count {
            if !servers && self.pool(pool).is_empty() {
                pool = Pool::Private;
            }

            address = next_address(address, 1);

            if start.is_none() {
                self.cursor = address;
            }

            if self.pool(pool).is_empty()
                || (!servers && (server_count >= 3 || self.next(0, 100) >= 80))
            {
                self.device_computer(device, address);
                continue;
            }

            server_count += 1;
            self.server(device, address, pool, servers, isolated);
        }
    }

    /// Creates a server for `device` at `address`, drawing services from `pool`.
    fn server(&mut self, device: usize, address: [u32; 4], pool: Pool, all: bool, isolated: bool) {
        let available = self.pool(pool).len() as u32;
        let mut count = if all {
            available + 1
        } else {
            self.next(1, available + 1)
        };
        let is_router = self.devices[device].is_router();

        if self.kind == NetworkKind::HackShop
            && is_router
            && self
                .pool(pool)
                .iter()
                .any(|entry| entry.service == ServiceId::Repository)
            && count == 1
        {
            count += 1;
        }

        let mut forwarded = is_router && self.next(0, 4) > 1;
        let mut choice: Vec<PoolService> = Vec::new();
        let mut drop_service: Vec<PoolService> = Vec::new();

        for _ in 0..count {
            let len = self.pool(pool).len() as u32;

            if len == 0 {
                break;
            }

            let index = self.next(0, len) as usize;
            let entry = self.pool(pool)[index];

            if choice.contains(&entry) {
                continue;
            }

            choice.push(entry);

            if repeatable(entry.service, self.kind) {
                if self.next(0, 24) == 0 {
                    self.pool(pool).retain(|other| *other != entry);
                }

                continue;
            }

            if main_service(entry.service, self.kind) {
                // `RevertGenComputer`: only router computers run network main services.
                if !is_router {
                    let back = std::mem::take(&mut drop_service);

                    self.pool(pool).extend(back);
                    choice.clear();
                    self.plain_computer(device, address);
                    break;
                }

                forwarded = true;
            }

            self.pool(pool).retain(|other| *other != entry);
            drop_service.push(entry);
        }

        if !choice.is_empty() {
            self.add_server(
                device,
                address,
                DeviceKind::Computer,
                &choice,
                forwarded && !isolated,
            );
        }
    }

    /// `ConfigDevices`: creates a standard computer, camera, or smart appliance at `address`.
    fn device_computer(&mut self, device: usize, address: [u32; 4]) {
        let mut kinds = vec![(DeviceKind::Computer, 70)];

        if self.cameras < 6 && self.kind != NetworkKind::Residential {
            kinds.push((DeviceKind::Camera, 35));

            if !self.fridge {
                kinds.push((DeviceKind::Fridge, 35));
                self.fridge = true;
            }

            if !self.microwave {
                kinds.push((DeviceKind::Microwave, 35));
                self.microwave = true;
            }
        }

        let total = kinds.iter().map(|(_, weight)| weight).sum();
        let hit = self.next(0, total);
        let mut sum = 0;
        let mut kind = DeviceKind::Computer;

        for (candidate, weight) in kinds {
            sum += weight;

            if hit < sum {
                kind = candidate;
                break;
            }
        }

        let services: &[ServiceId] = match kind {
            DeviceKind::Camera => &[ServiceId::Http, ServiceId::Cam],
            DeviceKind::Fridge | DeviceKind::Microwave => &[ServiceId::SmartAppliance],
            _ => {
                self.plain_computer(device, address);

                return;
            }
        };
        let services: Vec<PoolService> = services
            .iter()
            .map(|service| PoolService {
                service: *service,
                chance: 0,
                closed: false,
            })
            .collect();

        if kind == DeviceKind::Camera {
            self.camera(device, address);

            return;
        }

        self.add_server(device, address, kind, &services, false);
    }

    /// `CreateCamera`: creates a camera with web and camera services at `address` under `device`.
    pub(super) fn camera(&mut self, device: usize, address: [u32; 4]) {
        let services: Vec<PoolService> = [ServiceId::Http, ServiceId::Cam]
            .into_iter()
            .map(|service| PoolService {
                service,
                chance: 0,
                closed: false,
            })
            .collect();

        self.cameras += 1;
        self.add_server(device, address, DeviceKind::Camera, &services, false);
    }

    /// Returns whether the plan includes a fridge or microwave (`NetworkLan.HasSmartAppliance`).
    pub(super) fn has_smart_appliance(&self) -> bool {
        self.devices.iter().any(|device| {
            device
                .computers
                .iter()
                .any(|computer| matches!(computer.kind, DeviceKind::Fridge | DeviceKind::Microwave))
        })
    }

    /// Returns the address after the highest address using `gateway`, including routers and
    /// switches in its subnet (`NetworkLan.GetNextIP`).
    pub(super) fn next_ip(&self, gateway: &str) -> [u32; 4] {
        let mut highest = parse(gateway);

        for device in &self.devices {
            let addresses = device
                .computers
                .iter()
                .filter(|computer| computer.gateway == gateway)
                .map(|computer| computer.local_ip.as_str())
                .chain(
                    (!device.local_ip.is_empty()
                        && device.local_ip != gateway
                        && same_subnet(&device.local_ip, gateway))
                    .then_some(device.local_ip.as_str()),
                );

            for address in addresses {
                let address = parse(address);

                if address[3] > highest[3] && address[..3] == highest[..3] {
                    highest = address;
                }
            }
        }

        next_address(highest, 1)
    }

    fn plain_computer(&mut self, device: usize, address: [u32; 4]) {
        self.attach(
            device,
            ComputerPlan {
                kind: DeviceKind::Computer,
                local_ip: format_address(address),
                gateway: String::new(),
                services: Vec::new(),
            },
        );
    }

    /// `PreCrearServidor`: creates a computer at `address` with `services`. Each service uses the
    /// first available `next_port` at or after its default; `forwarded` adds main-router forwards.
    fn add_server(
        &mut self,
        device: usize,
        address: [u32; 4],
        kind: DeviceKind,
        services: &[PoolService],
        forwarded: bool,
    ) {
        let lan_ip = format_address(address);
        let mut services_of_server: Vec<ServicePlan> = Vec::new();

        for entry in services {
            let mut port = entry.service.default_port();

            while services_of_server.iter().any(|other| other.port == port) {
                port += 1;
            }

            services_of_server.push(ServicePlan {
                service: entry.service,
                port,
                closed: entry.closed,
            });

            if forwarded {
                let number = self.external_port(port, entry.service);

                self.table.push(ForwardLayout {
                    number,
                    lan_ip: lan_ip.clone(),
                    port,
                });

                if !self.devices[device].is_hub() {
                    self.router_forward_addresses.push(lan_ip.clone());
                }
            }
        }

        self.attach(
            device,
            ComputerPlan {
                kind,
                local_ip: lan_ip,
                gateway: String::new(),
                services: services_of_server,
            },
        );
    }

    /// `Router.GetNextExternalPort`: returns the first unused public port at or after `port`.
    /// Starts randomly for drop_service except SQL services.
    pub(super) fn external_port(&mut self, port: u16, service: ServiceId) -> u16 {
        let next_port = |table: &[ForwardLayout], from: u16| {
            let mut number = from;

            while table.iter().any(|forward| forward.number == number) {
                number = number.wrapping_add(1);
            }

            number
        };
        let number = next_port(&self.table, port);

        if number != port
            && !matches!(
                service,
                ServiceId::Students | ServiceId::Criminals | ServiceId::Employees
            )
        {
            let random = self.next(1024, 65501) as u16;

            return next_port(&self.table, random);
        }

        number
    }

    /// `ConfigGateway`: places `computer` under `device`. Uses the device private gateway, the
    /// device itself for routers and switches, or waits for router adoption under a hub.
    pub(super) fn attach(&mut self, device: usize, mut computer: ComputerPlan) {
        let holder = &self.devices[device];

        if !holder.private_gateway.is_empty() {
            computer.gateway = holder.private_gateway.clone();
        } else if holder.is_router() || holder.is_switch() {
            computer.gateway = holder.local_ip.clone();
        }

        let waits = computer.gateway.is_empty() && holder.is_hub();

        self.devices[device].computers.push(computer);

        if waits {
            let index = self.devices[device].computers.len() - 1;

            self.pending_hub.push((device, index));
        }
    }
}
