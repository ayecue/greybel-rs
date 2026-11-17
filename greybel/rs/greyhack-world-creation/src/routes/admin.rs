//! Network creation through the global `get_greybel_util()` program.

use greyhack_world_core::RouterId;
use greyhack_world_programs::protocol::{
    AdminCreateNetwork, AdminCreateProceduralMission, AdminCreateZeroDayChain,
    AdminRequestZeroDayMission, AdminVisitZeroDayHackShop, Failure, NetworkCreation,
    NetworkOptions, ZeroDayMission, ZeroDayMissionInfo,
};
use greyhack_world_router::RouteTable;

use super::Result;
use super::protocol::Creation;
use crate::ObjectTableExt as _;
use crate::services::zero_day::RumorRequest;

pub(crate) fn register<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: AdminCreateNetwork| {
            Ok(
                match create_network(&*ctx.modules, message.network).await? {
                    Creation::Object(router) => {
                        NetworkCreation::Router(ctx.trees.objects(call.tree).add_router(router))
                    }
                    Creation::Failure(failure) => NetworkCreation::Failure(Failure::Admin(failure)),
                },
            )
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    _message: AdminCreateZeroDayChain| {
            Ok(ctx
                .modules
                .module::<crate::CreationModule>()
                .zero_day
                .create_zero_day_chain()
                .await?)
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    message: AdminCreateProceduralMission| {
            Ok(ctx
                .modules
                .module::<crate::CreationModule>()
                .zero_day
                .create_procedural_mission(message.seed)
                .await?)
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    _message: AdminRequestZeroDayMission| {
            Ok(
                match ctx
                    .modules
                    .module::<crate::CreationModule>()
                    .zero_day
                    .request_zero_day_mission()
                    .await?
                {
                    RumorRequest::Failure(text) => ZeroDayMission::Failure(text),
                    RumorRequest::Rumor(rumor) => ZeroDayMission::Rumor(ZeroDayMissionInfo {
                        public_ip: rumor.public_ip,
                        lan_ip: rumor.lan_ip,
                        sender: rumor.sender,
                        body: rumor.body,
                    }),
                },
            )
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    message: AdminVisitZeroDayHackShop<'static>| {
            Ok(ctx
                .modules
                .module::<crate::CreationModule>()
                .zero_day
                .hack_shop_rumor_available(&message.public_ip)
                .await?)
        },
    );
}

/// Creates a network if neither its public address nor domain is already registered.
async fn create_network(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    network: NetworkOptions,
) -> Result<Creation<RouterId>> {
    let core = world.module::<greyhack_world_core::CoreModule>();

    if let Some(address) = &network.public_ip
        && core.networks.routers.router_at(address).await?.is_some()
    {
        return Ok(Creation::Failure(
            greyhack_world_programs::protocol::AdminFailure::AddressTaken(address.clone()),
        ));
    }

    if let Some(domain) = &network.domain
        && core.networks.routers.lookup(domain).await?.is_some()
    {
        return Ok(Creation::Failure(
            greyhack_world_programs::protocol::AdminFailure::DomainTaken(domain.clone()),
        ));
    }

    let router = world
        .module::<crate::CreationModule>()
        .network_generation
        .add_network(&network.into())
        .await?;

    Ok(Creation::Object(router))
}
