//! Service control operations (`ServiceIntrinsics`) restricted to root.

use std::rc::Rc;

use duplex_core::{Channel, Driver};
use greyhack_world::{ServiceControlFailure as WorldFailure, ServiceStart};

use crate::connection::Connection;
use crate::protocol::{
    Failure, GameServer, Outcome, ServiceControlFailure, ServiceInstallService,
    ServiceStartService, ServiceStopService,
};

pub(super) fn attach<D: Driver>(connection: &Rc<Connection>, channel: &Channel<GameServer, D>) {
    connection.change(
        channel,
        async |world, objects, message: ServiceInstallService, _| {
            let service = objects.service(message.service)?;

            Ok(
                match world
                    .control_install_service(
                        service.access.computer,
                        &service.access.user,
                        service.service,
                    )
                    .await?
                {
                    Ok(success) => success.into(),
                    Err(failure) => refusal(failure),
                },
            )
        },
    );
    connection.change(
        channel,
        async |world, objects, message: ServiceStartService, _| {
            let service = objects.service(message.service)?;

            Ok(
                match world
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
    connection.change(
        channel,
        async |world, objects, message: ServiceStopService, _| {
            let service = objects.service(message.service)?;

            Ok(
                match world
                    .control_stop_service(
                        service.access.computer,
                        &service.access.user,
                        service.service,
                    )
                    .await?
                {
                    Ok(success) => success.into(),
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
