//! Network administrator reports of newly released libraries (`adminNetwork`).

use greyhack_world_router::RouteTable;

use crate::LibraryReportFailure;
use crate::routes::protocol::{AdminFailure, AdminReportLibraryUpdate, Failure, Outcome};

pub(crate) fn register<W: crate::LibraryContext + 'static>(routes: &RouteTable) {
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    message: AdminReportLibraryUpdate<'static>| {
            Ok(
                match ctx
                    .modules
                    .libraries()
                    .updates
                    .report_library_update(&message.public_ip, &message.library)
                    .await?
                {
                    None => Outcome::Null,
                    Some(Ok(())) => Outcome::True,
                    Some(Err(LibraryReportFailure::UnknownLibrary)) => Outcome::False,
                    Some(Err(LibraryReportFailure::AdminRestoreOff)) => {
                        Outcome::Failure(Failure::Admin(AdminFailure::LibraryUpdateSkipped))
                    }
                },
            )
        },
    );
}
