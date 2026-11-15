//! Computer creation through a single-network `admin.so` object.

use greyhack_behaviour::ROOT_USER;
use greyhack_world_core::{Access, ComputerId};
use greyhack_world_router::RouteTable;
use greyhack_world_router::protocol::{
    AdminAddComputer, AdminFailure, ComputerCreation, ComputerOptions, Failure,
};

use super::Result;
use super::protocol::Creation;
use crate::ObjectTableExt as _;

pub(crate) fn register<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: AdminAddComputer<'static>| {
            Ok(
                match add_computer(&*ctx.modules, &message.public_ip, message.computer).await? {
                    Creation::Object(computer) => ComputerCreation::Computer(
                        ctx.trees.objects(call.tree).add_computer(Access {
                            computer,
                            user: ROOT_USER.to_owned(),
                            ftp: false,
                            prev_ip: String::new(),
                            bounces: 0,
                        }),
                    ),
                    Creation::Failure(failure) => {
                        ComputerCreation::Failure(Failure::Admin(failure))
                    }
                },
            )
        },
    );
}

/// Creates a computer on the network at `public_ip`.
async fn add_computer(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    public_ip: &str,
    computer: ComputerOptions,
) -> Result<Creation<ComputerId>> {
    let Some(router) = world
        .module::<greyhack_world_core::CoreModule>()
        .networks
        .routers
        .router_at(public_ip)
        .await?
        .map(|router| router.id)
    else {
        return Ok(Creation::Failure(AdminFailure::UnknownNetwork(
            public_ip.to_owned(),
        )));
    };

    Ok(world
        .module::<crate::CreationModule>()
        .network_generation
        .add_computer_to(router, &computer.into())
        .await?
        .map_or_else(
            || Creation::Failure(AdminFailure::LanAddressTaken),
            Creation::Object,
        ))
}
