//! Guild Admin Restore operations of the `adminNetwork` object.

use greyhack_world_programs::protocol::{AdminSetAdminRestore, Failure, Outcome};
use greyhack_world_router::RouteTable;

use crate::routes::protocol::IntoProtocol as _;

pub(crate) fn register<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    message: AdminSetAdminRestore<'static>| {
            Ok(
                match ctx
                    .modules
                    .module::<crate::GuildModule>()
                    .guilds
                    .set_admin_restore(&message.guild, &message.public_ip, message.enabled)
                    .await?
                {
                    Ok(()) => Outcome::True,
                    Err(failure) => Outcome::Failure(Failure::Admin(failure.into_protocol())),
                },
            )
        },
    );
}
