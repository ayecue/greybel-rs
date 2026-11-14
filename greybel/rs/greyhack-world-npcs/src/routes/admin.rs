//! NPC operations of the global `admin.so` program.

use greyhack_world_router::RouteTable;
use greyhack_world_router::protocol::AdminCreateMailAccount;

use crate::NpcContext;

pub(crate) fn register<W: NpcContext + 'static>(routes: &RouteTable) {
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    message: AdminCreateMailAccount<'static>| {
            ctx.modules
                .npcs()
                .mail
                .add_mail_account(&message.address, &message.password)
                .await?;

            Ok(true)
        },
    );
}
