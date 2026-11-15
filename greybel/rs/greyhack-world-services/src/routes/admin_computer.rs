//! Service operations of the single-computer `admin.so` object.

use greyhack_behaviour::ServiceId;
use greyhack_world_programs::protocol::{
    AdminFailure, AdminInstallSmartphoneAdb, AdminServiceRecords, AdminUpdateServiceRecord,
    Failure, Outcome,
};
use greyhack_world_router::RouteTable;

use crate::routes::Result;
use crate::{Record, RecordFailure};

pub(crate) fn register<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    message: AdminServiceRecords<'static>| {
            service_records(&*ctx.modules, &message).await
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    message: AdminUpdateServiceRecord<'static>| {
            update_service_record(&*ctx.modules, &message).await
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    message: AdminInstallSmartphoneAdb<'static>| {
            let world = &*ctx.modules;
            let Some(computer) = world
                .module::<greyhack_world_core::CoreModule>()
                .networks
                .routers
                .computer_at(&message.public_ip, &message.lan_ip)
                .await?
            else {
                return Ok(false);
            };

            Ok(world
                .module::<crate::ServicesModule>()
                .installs
                .install_smartphone_adb(computer)
                .await?)
        },
    );
}

/// Returns service records as UI-viewer JSON (`FileHelperServer.LeerFichaServerRpc`).
async fn service_records(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    message: &AdminServiceRecords<'_>,
) -> Result<Option<String>> {
    let Some(computer) = world
        .module::<greyhack_world_core::CoreModule>()
        .networks
        .routers
        .computer_at(&message.public_ip, &message.lan_ip)
        .await?
    else {
        return Ok(None);
    };
    let Some(service) = ServiceId::from_name(&message.service) else {
        return Ok(None);
    };

    Ok(world
        .module::<crate::ServicesModule>()
        .installs
        .service_records(computer, service)
        .await?
        .map(|records| records.to_json()))
}

/// Applies a UI-viewer record update and returns any failure (`FileHelperServer.UpdateFichaServerRpc`).
async fn update_service_record(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    message: &AdminUpdateServiceRecord<'_>,
) -> Result<Outcome> {
    let failure = |failure| Ok(Outcome::Failure(Failure::Admin(failure)));
    let Some(computer) = world
        .module::<greyhack_world_core::CoreModule>()
        .networks
        .routers
        .computer_at(&message.public_ip, &message.lan_ip)
        .await?
    else {
        return failure(AdminFailure::MissingRecord);
    };
    let Some(service) = ServiceId::from_name(&message.service) else {
        return failure(AdminFailure::CorruptedRecords);
    };
    let Some(record) = Record::parse_for(service, &message.record) else {
        return failure(AdminFailure::CorruptedRecords);
    };

    match world
        .module::<crate::ServicesModule>()
        .installs
        .update_service_record(computer, service, record)
        .await?
    {
        Ok(()) => Ok(Outcome::True),
        Err(RecordFailure::Unreadable) => failure(AdminFailure::CorruptedRecords),
        Err(RecordFailure::MissingEntry) => failure(AdminFailure::MissingRecord),
    }
}
