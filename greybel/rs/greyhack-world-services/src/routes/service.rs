//! Service control operations (`ServicesIntrinsics`) restricted to root.

use greyhack_world_router::RouteTable;
use greyhack_world_router::protocol::{
    Failure, Outcome, ServiceControlFailure, ServiceInstallService, ServiceStartService,
    ServiceStopService,
};

use crate::objects::ObjectTableExt;
use crate::routes::protocol::IntoProtocol as _;
use crate::{ServiceControlFailure as WorldFailure, ServiceStart};

pub(crate) fn register<W: crate::ServicesContext + 'static>(routes: &RouteTable) {
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: ServiceInstallService| {
            let world = &*ctx.modules;
            let service = &ctx.trees.objects(call.tree).service(message.service)?;

            Ok(
                match world
                    .services()
                    .control
                    .control_install_service(
                        service.access.computer,
                        &service.access.user,
                        service.service,
                    )
                    .await?
                {
                    Ok(success) => success.into_protocol(),
                    Err(failure) => refusal(failure),
                },
            )
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: ServiceStartService| {
            let world = &*ctx.modules;
            let service = &ctx.trees.objects(call.tree).service(message.service)?;

            Ok(
                match world
                    .services()
                    .control
                    .control_start_service(
                        service.access.computer,
                        &service.access.user,
                        service.service,
                    )
                    .await?
                {
                    Ok(ServiceStart::Gone) => Outcome::False,
                    Ok(ServiceStart::Success) => Outcome::True,
                    Ok(ServiceStart::Failure) => Outcome::Null,
                    Err(failure) => refusal(failure),
                },
            )
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: ServiceStopService| {
            let world = &*ctx.modules;
            let service = &ctx.trees.objects(call.tree).service(message.service)?;

            Ok(
                match world
                    .services()
                    .control
                    .control_stop_service(
                        service.access.computer,
                        &service.access.user,
                        service.service,
                    )
                    .await?
                {
                    Ok(success) => success.into_protocol(),
                    Err(failure) => refusal(failure),
                },
            )
        },
    );
}

/// Converts a refusal service operation to an outcome.
fn refusal(failure: WorldFailure) -> Outcome {
    Outcome::Failure(Failure::ServiceControl(match failure {
        WorldFailure::RootOnly => ServiceControlFailure::RootOnly,
        WorldFailure::RouterHttp => ServiceControlFailure::RouterHttp,
    }))
}
