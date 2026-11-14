use greybel_test_support::future::block_on;
use greyhack_behaviour::ServiceId;
use greyhack_world::World;

use super::read_routers;

/// A service a router of another network forwards, with an account of its computer.
pub struct Target {
    pub router: greyhack_world::RouterId,
    pub computer: greyhack_world::ComputerId,
    pub devices: Vec<greyhack_world::ComputerId>,
    pub address: String,
    pub port: u16,
    pub service: String,
    pub user: String,
    pub password: String,
}

pub fn remote_service(world: &World) -> Target {
    for router in read_routers(world).into_iter().skip(1) {
        let devices = block_on(world.modules.core.networks.topology.devices(router.id)).unwrap();

        for forward in block_on(
            world
                .modules
                .core
                .networks
                .router_config
                .port_forwards(router.id),
        )
        .unwrap()
        {
            if forward.port.service != ServiceId::Ssh {
                continue;
            }

            let user = block_on(world.modules.core.computers.users.users(forward.computer))
                .unwrap()[0]
                .clone();

            return Target {
                router: router.id,
                computer: forward.computer,
                devices,
                address: router.public_ip,
                port: forward.number,
                service: forward.port.service.name().to_owned(),
                user: user.name,
                password: user.password,
            };
        }
    }

    panic!("a remote SSH service");
}
