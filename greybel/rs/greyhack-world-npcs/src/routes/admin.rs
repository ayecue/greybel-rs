//! NPC operations of the global `get_greybel_util()` program.

use greyhack_world_programs::protocol::AdminCreateMailAccount;
use greyhack_world_router::RouteTable;

pub(crate) fn register<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    message: AdminCreateMailAccount<'static>| {
            ctx.modules
                .module::<crate::NpcModule>()
                .mail
                .add_mail_account(&message.address, &message.password)
                .await?;

            Ok(true)
        },
    );
}
