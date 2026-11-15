//! Administrator operations that create or change library files on a computer.

use greyhack_behaviour::{FileData, PermissionSet, path};
use greyhack_world_core::{ComputerId, NodeDraft, ObjectTableExt as _};
use greyhack_world_programs::protocol::{
    AdminCreateProgram, AdminSetLibraryVersion, AdminSetPortLibraryVersion,
};
use greyhack_world_router::{RouteContext, RouteTable};

pub(super) fn register<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &RouteContext<'_, W>, _call, message: AdminSetLibraryVersion<'static>| {
            let world = &*ctx.modules;
            let Some(computer) = computer_at(world, &message.public_ip, &message.lan_ip).await?
            else {
                return Ok(false);
            };

            Ok(world
                .module::<greyhack_world_core::CoreModule>()
                .computers
                .hardware
                .set_library_version(computer, &message.path, &message.version)
                .await?)
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &RouteContext<'_, W>,
                    _call,
                    message: AdminSetPortLibraryVersion<'static>| {
            let world = &*ctx.modules;
            let Some(computer) = computer_at(world, &message.public_ip, &message.lan_ip).await?
            else {
                return Ok(false);
            };

            Ok(world
                .module::<greyhack_world_core::CoreModule>()
                .computers
                .hardware
                .set_port_library_version(computer, message.port, &message.version)
                .await?)
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &RouteContext<'_, W>, call, message: AdminCreateProgram<'static>| {
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
}

async fn computer_at(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    public_ip: &str,
    lan_ip: &str,
) -> super::super::Result<Option<ComputerId>> {
    Ok(world
        .module::<greyhack_world_core::CoreModule>()
        .networks
        .routers
        .computer_at(public_ip, lan_ip)
        .await?)
}

async fn create_program(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    computer: ComputerId,
    user: &str,
    path: &str,
    source: &str,
) -> super::super::Result<bool> {
    let Some(folder) = world
        .module::<greyhack_world_core::CoreModule>()
        .files
        .lookup
        .folder(computer, path::parent(path), true)
        .await?
    else {
        return Ok(false);
    };
    let node = NodeDraft::file(
        path::file_name(path),
        user,
        PermissionSet::default_for(false),
        FileData::script(source),
    );

    world
        .module::<crate::LibraryModule>()
        .generator
        .add_full_node(&folder, &node)
        .await?;

    Ok(true)
}
