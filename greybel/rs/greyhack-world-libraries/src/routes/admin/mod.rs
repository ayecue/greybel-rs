//! Library versions, exploit reports and purchases, and debug accounts (`admin.so`).

use greyhack_world_core::ObjectTableExt as _;
use greyhack_world_router::RouteTable;

use crate::DebugAccount;
use crate::routes::protocol::{
    AdminBuyExploit, AdminCreateDebugAccount, AdminExploitShop, AdminReleaseLibraryVersion,
    AdminReportExploit,
};

mod library_versions;
mod shop;

use self::library_versions::{release_library_version, report_exploit};
use self::shop::{buy_exploit, exploit_shop};

pub(crate) fn register<W: crate::LibraryContext + 'static>(routes: &RouteTable) {
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    message: AdminReleaseLibraryVersion<'static>| {
            release_library_version(&*ctx.modules, &message.library).await
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    message: AdminReportExploit<'static>| {
            report_exploit(
                &*ctx.modules,
                &message.library,
                &message.zone,
                &message.unsec_value,
            )
            .await
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    message: AdminExploitShop<'static>| {
            exploit_shop(&*ctx.modules, &message.library, &message.version).await
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: AdminBuyExploit<'static>| {
            let session = &ctx.trees.objects(call.tree).session(
                ctx.session(call)
                    .expect("dispatch checked the active program"),
            )?;

            buy_exploit(
                &*ctx.modules,
                session.computer,
                &session.user,
                &ctx.terminal_computers(call),
                &message,
            )
            .await
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    message: AdminCreateDebugAccount<'static>| {
            ctx.modules
                .libraries()
                .debug
                .save_debug_account(&DebugAccount::new(&message.user, &message.password))
                .await?;

            Ok(true)
        },
    );
}
