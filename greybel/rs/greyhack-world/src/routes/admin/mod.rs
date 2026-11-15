//! Global `admin.so` operations for clocks, random seeds, and snapshots.

use greyhack_world_router::RouteTable;

use crate::routes::CyclesExt;
use crate::routes::conversions::{AdminSeed, AdminSetDate, AdminSnapshot, AdminTick};

pub(crate) fn register(routes: &RouteTable) {
    routes.read::<_, _, crate::World, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, crate::World>,
                    _call,
                    _message: AdminSeed| {
            let world = &*ctx.modules;

            Ok(world.handle.world().seed)
        },
    );
    routes.wait::<_, _, crate::World, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, crate::World>,
                    call,
                    message: AdminTick| {
            let cycles = ctx.cycles();
            let program = call.program;
            let target = cycles
                .change(
                    program,
                    async move |ctx: &greyhack_world_router::RouteContext<'_, crate::World>,
                                _call| {
                        let world = &*ctx.modules;

                        Ok(world.handle.time_seconds().saturating_add(message.seconds))
                    },
                )
                .await?;
            let budget = cycles.routine_budget();

            // Advance each cycle to the next due routine or the target, then run due routines.
            while !cycles
                .world(async |world| world.advance_step(target, budget).await)
                .await?
            {}

            Ok(cycles
                .world(async |world| world.handle.time_seconds())
                .await)
        },
    );
    routes.change::<_, _, crate::World, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, crate::World>,
                    _call,
                    message: AdminSetDate<'static>| {
            let world = &*ctx.modules;

            Ok(world.modules.routines.set_date(&message.date))
        },
    );
    routes.wait::<_, _, crate::World, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, crate::World>,
                    _call,
                    _: AdminSnapshot| { Ok(ctx.cycles().save().await) },
    );
}
