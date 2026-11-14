//! Guild Admin Restore operations of the `adminNetwork` object.

use greyhack_world_router::RouteTable;
use greyhack_world_router::protocol::{AdminSetAdminRestore, Failure, Outcome};

use crate::GuildContext;
use crate::routes::protocol::IntoProtocol as _;

pub(crate) fn register<W: GuildContext + 'static>(routes: &RouteTable) {
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    message: AdminSetAdminRestore<'static>| {
            Ok(
                match ctx
                    .modules
                    .guilds()
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
