//! Flooding a network session's target (`NetSessionIntrinsics.flood_connection`).

use greyhack_world_router::{CycleAccess, RouteTable};

use crate::routes::OperationError;
use crate::routes::protocol::{
    FloodResult, FloodWarning as WarningKind, NetSessionFloodConnection, PrintLine,
};
use crate::{Flood, FloodWarning};

pub(crate) fn register<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.wait::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: NetSessionFloodConnection| {
            let cycles = ctx.cycles();
            let program = call.program;

            CycleAccess::change_dynamic(
                cycles,
                program,
                false,
                async move |ctx: &greyhack_world_router::RouteContext<'_, W>, call| {
                    let world = &*ctx.modules;
                    let intrusion = world.module::<crate::IntrusionModule>();
                    let objects = ctx.trees.objects(call.tree);
                    let Some((computer, origin)) = intrusion
                        .library_objects
                        .flood_session(&objects, message.net_session)
                        .await?
                    else {
                        return Ok::<_, OperationError>(FloodResult::Missing);
                    };
                    let attempt = intrusion.floods.flood_from(computer, origin).await?;
                    let address = attempt.address;
                    let warning = match attempt.warning {
                        None => WarningKind::None,
                        Some(FloodWarning::SuperAdmin) => WarningKind::SuperAdmin,
                        Some(FloodWarning::CrownShield) => WarningKind::CrownShield,
                    };

                    ctx.print(call, PrintLine::FloodStart { address, warning });

                    Ok(match attempt.flood {
                        Flood::Active(work) => {
                            // Flood work continues until program termination cancels it (`WaitResultDownload`).
                            drop(CycleAccess::defer_dynamic::<
                                (),
                                W,
                                greyhack_world_core::WorldError,
                            >(cycles, program, work));

                            FloodResult::Active
                        }
                        Flood::Shield => FloodResult::Active,
                        Flood::Reboot => {
                            ctx.print(call, PrintLine::FloodInterrupted);

                            FloodResult::Reboot
                        }
                    })
                },
            )
            .await
        },
    );
}
