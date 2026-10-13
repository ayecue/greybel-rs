//! Operations on smart appliances (`SmartApplianceIntrinsics`).

use std::rc::Rc;

use duplex_core::{Channel, Driver};

use crate::connection::Connection;
use crate::protocol::{
    ApplianceModel, Failure, GameServer, SmartApplianceModel, SmartApplianceOverrideSettings,
    SmartApplianceSetAlarm,
};

pub(super) fn attach<D: Driver>(connection: &Rc<Connection>, channel: &Channel<GameServer, D>) {
    connection.read(
        channel,
        async |world, objects, message: SmartApplianceModel, _| {
            let computer = objects.smart_appliance(message.appliance)?.computer;

            Ok(match world.appliance_model(computer).await? {
                Ok(model) => ApplianceModel::Model(model),
                Err(failure) => ApplianceModel::Failure(Failure::Device(failure.into())),
            })
        },
    );
    connection.change(
        channel,
        async |world, objects, message: SmartApplianceOverrideSettings, _| {
            let computer = objects.smart_appliance(message.appliance)?.computer;

            Ok(world
                .override_appliance(computer, message.power, message.temperature)
                .await?
                .into())
        },
    );
    connection.change(
        channel,
        async |world, objects, message: SmartApplianceSetAlarm, _| {
            let computer = objects.smart_appliance(message.appliance)?.computer;

            Ok(world
                .set_appliance_alarm(computer, message.enable)
                .await?
                .into())
        },
    );
}
