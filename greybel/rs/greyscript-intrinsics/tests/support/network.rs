use greybel_test_support::future::block_on;
use greyhack_behaviour::ServiceId;
use greyhack_world::World;
use greyhack_world_core::{ComputerId, LoginFailure, RouterId};

use super::read_routers;

/// The seed of a generated world whose one network forwards SSH and FTP from outside.
pub const SERVICE_SEED: u64 = 9;

pub fn literal(text: &str) -> String {
    format!("\"{}\"", text.replace('"', "\"\""))
}

pub struct ServiceTarget {
    pub router: RouterId,
    pub computer: ComputerId,
    pub address: String,
    pub port: u16,
    pub service: String,
    pub user: String,
    pub password: String,
}

pub fn first_remote_service(world: &World) -> ServiceTarget {
    // Typed networks give computers different services, so find an SSH or FTP service a router forwards.
    let player = world.handle.player();

    for router in read_routers(world) {
        let devices = block_on(world.modules.core.networks.topology.devices(router.id))
            .expect("the world reads");

        if devices.contains(&player) {
            continue;
        }

        for forward in block_on(
            world
                .modules
                .core
                .networks
                .router_config
                .port_forwards(router.id),
        )
        .expect("the world reads")
        {
            if !matches!(forward.port.service, ServiceId::Ssh | ServiceId::Ftp) {
                continue;
            }

            let user = block_on(world.modules.core.computers.users.users(forward.computer))
                .expect("the world reads")[0]
                .clone();

            return ServiceTarget {
                router: router.id,
                computer: forward.computer,
                address: router.public_ip.clone(),
                port: forward.number,
                service: forward.port.service.name().to_owned(),
                user: user.name,
                password: user.password,
            };
        }
    }

    panic!("an SSH or FTP service");
}

/// A world with one network whose first computer's service is [`first_remote_service`], rebooting.
pub fn rebooted_service() -> (World, ServiceTarget) {
    let world = block_on(World::generate(&crate::support::world_options(
        SERVICE_SEED,
        1,
    )))
    .expect("a generated world");
    let target = first_remote_service(&world);

    block_on(
        world
            .modules
            .core
            .computers
            .processes
            .reboot(target.computer, false),
    )
    .unwrap();

    (world, target)
}

/// Connects the player's computer to `target`'s service, giving the computer or why not.
pub fn connect_to(world: &mut World, target: &ServiceTarget) -> Result<ComputerId, LoginFailure> {
    let player = world.handle.player();

    block_on(world.modules.core.networks.connect.connect_service(
        player,
        &target.address,
        target.port,
        &target.service,
        &target.user,
        &target.password,
        &[],
    ))
    .unwrap()
}
