//! Planning the devices of a LAN: the main router, LAN routers, hubs, subnets, Wi-Fi, and firewalls.

use greyhack_behaviour::{DeviceKind, LOCAL_RANGES, NetworkKind, ROUTER_PORT};

use super::Planner;
use super::addresses::{format_address, next_address, parse};
use super::post_generation::rule;
use crate::generation::lan::DevicePlan;
use crate::generation::network;

impl Planner<'_> {
    /// Creates the main-router device (`RouterNet(router, globalLan, isMainRouter: true)`).
    pub(super) fn main_router(&mut self) {
        let table = self.kind.public_services();

        self.public = self.draw_services(&table);
        self.devices[0].wifi = Some((String::new(), String::new()));

        let computers = if self.kind == NetworkKind::RentServer {
            1
        } else {
            self.next(1, 4)
        };

        self.generate(0, computers, None, true);
    }

    /// `Device.Generate`: creates child hubs, switches, routers, and computers.
    fn generate(&mut self, device: usize, computers: u32, start: Option<[u32; 4]>, main: bool) {
        let min = u32::from(main);
        let mut hubs = self.next(min, 4);
        let mut routers = self.next(min, 2);

        if self.total_devices >= self.max_devices {
            hubs = 0;
            routers = 0;
        }

        self.total_devices += hubs + routers;

        let mut computers = computers;

        if computers == 0 && hubs == 0 && routers == 0 {
            computers = 1;
        }

        self.computers(device, computers, hubs == 0 && routers == 0, start);

        for _ in 0..hubs {
            let hub = self.hub(device);

            self.devices[device].hubs.push(hub);
        }

        for _ in 0..routers {
            let router = self.lan_router(device);

            self.devices[device].routers.push(router);
        }
    }

    fn add_device(&mut self, kind: DeviceKind, parent: usize) -> usize {
        self.devices.push(DevicePlan::new(kind, "", Some(parent)));

        self.devices.len() - 1
    }

    /// Creates a LAN router (`RouterNet(router, globalLan)`).
    fn lan_router(&mut self, parent: usize) -> usize {
        let device = self.add_device(DeviceKind::Router, parent);
        let computers = self.next(0, 4);
        let mut start = None;

        self.cursor = next_address(self.cursor, 1);

        if !self.pending_gateways.is_empty() {
            let ((holder, index), count) = self.pending_gateways.remove(0);
            let gateway_ip = parse(&self.devices[holder].computers[index].local_ip);

            self.cursor = gateway_ip;
            self.cursor[3] = count + computers + 2;
            self.devices[holder].computers[index].gateway = format_address(self.cursor);
        } else if !self.pending_hub.is_empty() {
            start = Some(self.adopt_hub_computers(device, computers));
        } else {
            self.new_subnet();
        }

        self.devices[device].local_ip = format_address(self.cursor);
        self.last_device = device;
        self.generate(device, computers, start, false);
        self.wifi(device);
        self.firewall(device);

        device
    }

    /// `services_of_server`: creates the final router to adopt remaining hub computers
    /// (`RouterNet(globalLan)`).
    pub(super) fn adoption_router(&mut self, parent: usize) -> usize {
        let device = self.add_device(DeviceKind::Router, parent);

        self.adopt_hub_computers(device, 1);

        device
    }

    /// `ConfigPendingRouter`: allocates a subnet for `device`, adopts waiting hub computers after
    /// `skip` addresses, and returns the last allocated address.
    fn adopt_hub_computers(&mut self, device: usize, skip: u32) -> [u32; 4] {
        self.new_subnet();

        let own = self.cursor;
        let mut address = next_address(own, skip);

        self.devices[device].local_ip = format_address(own);

        for (holder, index) in std::mem::take(&mut self.pending_hub) {
            address = next_address(address, 1);

            let old = std::mem::replace(
                &mut self.devices[holder].computers[index].local_ip,
                format_address(address),
            );

            self.devices[holder].computers[index].gateway = format_address(own);

            // `UpdateRouterPortForward`: updates main-router forwards except those belonging to
            // router computers.
            if !self.router_forward_addresses.contains(&old) {
                for forward in &mut self.table {
                    if forward.lan_ip == old {
                        forward.lan_ip = format_address(address);
                    }
                }
            }
        }

        self.wifi(device);

        address
    }

    /// Creates a hub, or a switch with its own subnet immediately after a router (`HubNet`).
    fn hub(&mut self, parent: usize) -> usize {
        let device = self.add_device(DeviceKind::Hub, parent);
        let computers = self.next(0, 4);

        if self.pending_hub.is_empty() && self.devices[self.last_device].is_router() {
            self.new_subnet();

            if self.next(0, 2) == 0 {
                self.devices[device].kind = DeviceKind::Switch;
                self.devices[device].local_ip = format_address(self.cursor);
            } else {
                self.cursor = next_address(self.cursor, 1);
            }
        }

        self.last_device = device;
        self.generate(device, computers, None, false);
        self.firewall(device);

        device
    }

    /// `GetNewPrivateSubRange` and `CreateGlobalIP`: moves the cursor to `.1` in an unused subnet.
    fn new_subnet(&mut self) {
        let subnet = self.new_subnet_base();

        self.cursor = next_address(subnet, 1);
    }

    pub(super) fn new_subnet_base(&mut self) -> [u32; 4] {
        let prefix = LOCAL_RANGES[self.next(0, 3) as usize];
        let mut third = self.next(0, 26);

        while self.subnets.contains(&format!("{prefix}.{third}.0")) {
            third += 1;
        }

        let subnet = format!("{prefix}.{third}.0");

        self.subnets.push(subnet.clone());

        parse(&subnet)
    }

    /// `ConfigWifi`: enables Wi-Fi on one third of routers.
    fn wifi(&mut self, device: usize) {
        if self.devices[device].is_router() && self.next(0, 3) == 0 {
            let bssid = self.rng.mac();
            let essid = network::wifi_name(self.rng);

            self.devices[device].wifi = Some((bssid, essid));
        }
    }

    /// `ConfigFirewall`: denies all traffic on half of routers and switches; half of those allow
    /// only their configuration page.
    fn firewall(&mut self, device: usize) {
        if self.devices[device].is_hub() || self.next(0, 2) != 0 {
            return;
        }

        self.devices[device]
            .firewall
            .push(rule(false, None, None, None));
        self.deny_devices.push(device);

        if self.next(0, 2) == 0 {
            let address = self.devices[device].local_ip.clone();

            self.devices[device]
                .firewall
                .push(rule(true, Some(ROUTER_PORT), None, Some(address)));
        }
    }
}
