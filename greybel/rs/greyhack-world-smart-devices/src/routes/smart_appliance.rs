//! Operations on smart appliances (`SmartApplianceIntrinsics`).

use greyhack_world_router::RouteTable;

use crate::SmartDevicesContext;
use crate::objects::ObjectTableExt;
use crate::routes::protocol::{
    ApplianceModel, Failure, IntoProtocol as _, Outcome, SmartApplianceModel,
    SmartApplianceOverrideSettings, SmartApplianceSetAlarm,
};

pub(crate) fn register<W: SmartDevicesContext + 'static>(routes: &RouteTable) {
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: SmartApplianceModel| {
            let computer = ctx
                .trees
                .objects(call.tree)
                .smart_appliance(message.appliance)?
                .computer;

            Ok(
                match ctx
                    .modules
                    .smart_devices()
                    .appliances
                    .appliance_model(computer)
                    .await?
                {
                    Ok(model) => ApplianceModel::Model(model),
                    Err(failure) => {
                        ApplianceModel::Failure(Failure::Device(failure.into_protocol()))
                    }
                },
            )
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: SmartApplianceOverrideSettings| {
            let computer = ctx
                .trees
                .objects(call.tree)
                .smart_appliance(message.appliance)?
                .computer;

            Ok(ctx
                .modules
                .smart_devices()
                .appliances
                .override_appliance(computer, message.power, message.temperature)
                .await?
                .map_or_else(
                    |failure| Outcome::Failure(Failure::Device(failure.into_protocol())),
                    |()| Outcome::True,
                ))
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: SmartApplianceSetAlarm| {
            let computer = ctx
                .trees
                .objects(call.tree)
                .smart_appliance(message.appliance)?
                .computer;

            Ok(ctx
                .modules
                .smart_devices()
                .appliances
                .set_appliance_alarm(computer, message.enable)
                .await?
                .map_or_else(
                    |failure| Outcome::Failure(Failure::Device(failure.into_protocol())),
                    |()| Outcome::True,
                ))
        },
    );
}
