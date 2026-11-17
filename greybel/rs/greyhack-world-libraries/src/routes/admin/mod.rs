//! Library versions, exploit reports and purchases (`get_greybel_util()`).

use greyhack_world_core::ObjectTableExt as _;
use greyhack_world_router::RouteTable;

use crate::routes::protocol::{
    AdminBuyExploit, AdminExploitShop, AdminReleaseLibraryVersion, AdminReportExploit,
};

mod computer;
mod library_versions;
mod shop;
mod zero_day;

use self::library_versions::{release_library_version, report_exploit};
use self::shop::{buy_exploit, exploit_shop};

pub(crate) fn register<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    computer::register::<W>(routes);
    zero_day::register::<W>(routes);
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
}
