//! Operations on police traffic networks (`TrafficNetIntrinsics`).

use std::rc::Rc;

use duplex_core::{Channel, Driver};

use crate::connection::Connection;
use crate::protocol::{
    CredentialsInfo, Failure, GameServer, TrafficNetCameraLinkSystem, TrafficNetGetCredentialsInfo,
    TrafficNetLocateVehicle,
};

pub(super) fn attach<D: Driver>(connection: &Rc<Connection>, channel: &Channel<GameServer, D>) {
    connection.read(
        channel,
        async |world, objects, message: TrafficNetCameraLinkSystem, _| {
            let computer = objects.traffic_net(message.traffic)?.computer;

            Ok(world.camera_link_system(computer).await?.into())
        },
    );
    connection.read(
        channel,
        async |world, objects, message: TrafficNetGetCredentialsInfo, _| {
            let computer = objects.traffic_net(message.traffic)?.computer;

            Ok(match world.camera_credentials(computer).await? {
                Ok(Some(npc)) => CredentialsInfo::Info {
                    role: npc.job_role.game_name().to_owned(),
                    name: npc.name,
                },
                Ok(None) => CredentialsInfo::Null,
                Err(failure) => CredentialsInfo::Failure(Failure::Device(failure.into())),
            })
        },
    );
    connection.change(
        channel,
        async |world, objects, message: TrafficNetLocateVehicle<'static>, _| {
            let computer = objects.traffic_net(message.traffic)?.computer;

            Ok(world
                .locate_vehicle_from(
                    computer,
                    &message.license_plate,
                    &message.password,
                    !message.throttled,
                )
                .await?
                .into())
        },
    );
}
