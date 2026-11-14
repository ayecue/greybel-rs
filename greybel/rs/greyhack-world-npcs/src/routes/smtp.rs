//! SMTP account listing (`CryptoIntrinsics.smtp_user_list`).

use greyhack_world_core::Access;
use greyhack_world_router::RouteTable;
use greyhack_world_router::protocol::{CryptoSmtpUserList, SmtpUserList};

use crate::NpcContext;
use crate::routes::Result;
use crate::routes::protocol::IntoProtocol as _;

pub(crate) fn register<W: NpcContext + 'static>(routes: &RouteTable) {
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: CryptoSmtpUserList<'static>| {
            let npcs = ctx.modules.npcs();
            let objects = ctx.trees.objects(call.tree);
            let access = npcs.smtp_access.access(&objects, message.crypto)?;

            smtp_user_list(&*ctx.modules, &access, &message.address, message.port).await
        },
    );
}

/// Lists SMTP accounts at `address:port` through `access`.
async fn smtp_user_list(
    world: &impl NpcContext,
    access: &Access,
    address: &str,
    port: u16,
) -> Result<SmtpUserList> {
    Ok(
        match world
            .npcs()
            .mail
            .smtp_users(access.computer, address, port)
            .await?
        {
            Ok(users) => SmtpUserList::Users(users),
            Err(failure) => SmtpUserList::Failure(failure.into_protocol()),
        },
    )
}
