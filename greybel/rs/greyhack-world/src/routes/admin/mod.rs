//! Global `admin.so` operations (`admin`) for clocks, networks, programs, credentials, and snapshots.

use greyhack_world_router::RouteTable;

use crate::CoreObjectTableExt as _;
use crate::routes::CyclesExt;
use crate::routes::conversions::{
    AdminCreateProgram, AdminNetworks, AdminRegisterPassword, AdminSeed, AdminSetDate,
    AdminSnapshot, AdminTick,
};

mod networks;
use self::networks::create_program;

pub(crate) fn register(routes: &RouteTable) {
    routes.read::<_, _, crate::World, crate::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, crate::World>,
                    _call,
                    _message: AdminSeed| {
            let world = &*ctx.modules;

            Ok(world.handle.world().seed)
        },
    );
    routes.read::<_, _, crate::World, crate::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, crate::World>,
                    _call,
                    _message: AdminNetworks| {
            let world = &*ctx.modules;

            Ok(world
                .modules
                .core
                .networks
                .routers
                .public_addresses()
                .await?)
        },
    );
    routes.wait::<_, _, crate::World, crate::WorldError>(
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
    routes.change::<_, _, crate::World, crate::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, crate::World>,
                    _call,
                    message: AdminSetDate<'static>| {
            let world = &*ctx.modules;

            Ok(world.modules.routines.set_date(&message.date))
        },
    );
    routes.change::<_, _, crate::World, crate::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, crate::World>,
                    call,
                    message: AdminCreateProgram<'static>| {
            let world = &*ctx.modules;
            let session = &ctx.trees.objects(call.tree).session(
                ctx.session(call)
                    .expect("dispatch checked the active program"),
            )?;

            create_program(
                world,
                session.computer,
                &session.user,
                &message.path,
                &message.source,
            )
            .await
        },
    );
    routes.wait::<_, _, crate::World, crate::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, crate::World>,
                    _call,
                    _: AdminSnapshot| { Ok(ctx.cycles().save().await) },
    );
    routes.change::<_, _, crate::World, crate::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, crate::World>,
                    _call,
                    message: AdminRegisterPassword<'static>| {
            let world = &*ctx.modules;

            Ok(world
                .modules
                .core
                .computers
                .users
                .register_password(&message.password)
                .await?)
        },
    );
}
