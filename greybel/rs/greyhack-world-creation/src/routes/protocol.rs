//! Conversions between administrator protocol options and generation options.

use greyhack_world_programs::protocol::{
    AdminFailure, ComputerOptions as ComputerOptionsMessage,
    NetworkOptions as NetworkOptionsMessage,
};

/// Created router or computer ID, or the creation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Creation<T> {
    Object(T),
    Failure(AdminFailure),
}

impl From<NetworkOptionsMessage> for crate::NetworkOptions {
    fn from(network: NetworkOptionsMessage) -> Self {
        Self {
            public_ip: network.public_ip,
            kind: None,
            allow_missions: true,
            essid: network.essid,
            wifi_password: network.wifi_password,
            domain: network.domain,
            position: None,
            computers: network.computers,
        }
    }
}

impl From<ComputerOptionsMessage> for crate::ComputerOptions {
    fn from(computer: ComputerOptionsMessage) -> Self {
        let hardware = computer.mining_hardware;

        Self {
            name: computer.name,
            lan_ip: computer.lan_ip,
            root_password: computer.root_password,
            users: computer.users.map(|users| {
                users
                    .into_iter()
                    .map(|user| (user.name, user.password))
                    .collect()
            }),
            services: computer.services,
            is_switch: computer.is_switch,
            miner_hardware: greyhack_world_core::MinerHardware {
                cpu_speed: hardware.cpu_speed,
                cpu_cores: hardware.cpu_cores,
                ram_model: hardware.ram_model,
                gpu_hashrate: hardware.gpu_hashrate,
            },
        }
    }
}
