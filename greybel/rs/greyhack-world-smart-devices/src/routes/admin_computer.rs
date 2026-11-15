//! Smart-appliance and CCTV operations of the single-computer `admin.so` object.

use greyhack_world_router::RouteTable;
use greyhack_world_router::protocol::{
    AdminAccessCctv, AdminAddSmartAppliance, AdminFailure, CctvAccess, Failure,
    SmartApplianceOptions,
};

use crate::routes::Result;
use crate::{CctvFailure, SmartApplianceSettings};

pub(crate) fn register<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    message: AdminAddSmartAppliance<'static>| {
            add_smart_appliance(
                &*ctx.modules,
                &message.public_ip,
                &message.lan_ip,
                &message.appliance,
            )
            .await
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    message: AdminAccessCctv<'static>| {
            access_cctv(&*ctx.modules, &message).await
        },
    );
}

/// Enables smart-appliance settings on the computer at `lan_ip` behind `public_ip`.
async fn add_smart_appliance(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    public_ip: &str,
    lan_ip: &str,
    appliance: &SmartApplianceOptions,
) -> Result<bool> {
    let Some(computer) = world
        .module::<greyhack_world_core::CoreModule>()
        .networks
        .routers
        .computer_at(public_ip, lan_ip)
        .await?
    else {
        return Ok(false);
    };
    let mut settings =
        SmartApplianceSettings::new(&appliance.model, appliance.power, appliance.temperature);

    settings.override_enabled = appliance.override_enabled;

    Ok(world
        .module::<crate::SmartDevicesModule>()
        .appliances
        .enable_smart_appliance(computer, &settings)
        .await?)
}

/// Handles browser CCTV access (`NetworkHelperServer.AccessWebServerRpc`).
async fn access_cctv(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    message: &AdminAccessCctv<'_>,
) -> Result<CctvAccess> {
    let Some(computer) = world
        .module::<greyhack_world_core::CoreModule>()
        .networks
        .routers
        .computer_at(&message.public_ip, &message.lan_ip)
        .await?
    else {
        return Ok(CctvAccess::Failure(Failure::Admin(
            AdminFailure::WrongCameraPassword,
        )));
    };

    Ok(
        match world
            .module::<crate::SmartDevicesModule>()
            .cctv
            .access_cctv(computer, message.password.as_deref())
            .await?
        {
            Ok(None) => CctvAccess::Prompt,
            Ok(Some(feed)) => CctvAccess::Data(feed.to_json()),
            Err(CctvFailure::WrongPassword) => {
                CctvAccess::Failure(Failure::Admin(AdminFailure::WrongCameraPassword))
            }
            Err(CctvFailure::NoExternalAccess) => {
                CctvAccess::Failure(Failure::Admin(AdminFailure::CamerasDisabled))
            }
        },
    )
}
