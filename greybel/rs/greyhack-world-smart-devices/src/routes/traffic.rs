//! Operations on police traffic networks (`TrafficNetIntrinsics`).

use greyhack_world_router::RouteTable;

use crate::SmartDevicesContext;
use crate::objects::ObjectTableExt;
use crate::routes::protocol::{
    CredentialsInfo, Failure, IntoProtocol as _, Outcome, TrafficNetCameraLinkSystem,
    TrafficNetGetCredentialsInfo, TrafficNetLocateVehicle,
};

pub(crate) fn register<W: SmartDevicesContext + 'static>(routes: &RouteTable) {
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: TrafficNetCameraLinkSystem| {
            let computer = ctx
                .trees
                .objects(call.tree)
                .traffic_net(message.traffic)?
                .computer;

            Ok(ctx
                .modules
                .smart_devices()
                .traffic
                .camera_link_system(computer)
                .await?
                .map_or_else(
                    |failure| Outcome::Failure(Failure::Device(failure.into_protocol())),
                    |()| Outcome::True,
                ))
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: TrafficNetGetCredentialsInfo| {
            let computer = ctx
                .trees
                .objects(call.tree)
                .traffic_net(message.traffic)?
                .computer;

            Ok(
                match ctx
                    .modules
                    .smart_devices()
                    .traffic
                    .camera_credentials(computer)
                    .await?
                {
                    Ok(Some(npc)) => CredentialsInfo::Info {
                        role: npc.job_role.game_name().to_owned(),
                        name: npc.name,
                    },
                    Ok(None) => CredentialsInfo::Null,
                    Err(failure) => {
                        CredentialsInfo::Failure(Failure::Device(failure.into_protocol()))
                    }
                },
            )
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: TrafficNetLocateVehicle<'static>| {
            let computer = ctx
                .trees
                .objects(call.tree)
                .traffic_net(message.traffic)?
                .computer;

            Ok(ctx
                .modules
                .smart_devices()
                .traffic
                .locate_vehicle_from(
                    computer,
                    &message.license_plate,
                    &message.password,
                    !message.throttled,
                )
                .await?
                .map_or_else(
                    |failure| Outcome::Failure(Failure::Device(failure.into_protocol())),
                    |()| Outcome::True,
                ))
        },
    );
}
