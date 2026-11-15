//! Shell-related global `admin.so` operations.

use greyhack_behaviour::ROOT_USER;
use greyhack_world_core::{Access, AccessHandle, AccessKind, ComputerId, ObjectTableExt as _};
use greyhack_world_programs::protocol::AdminShell;
use greyhack_world_router::{CycleAccess, RouteContext, RouteTable};

pub(crate) fn register<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &RouteContext<'_, W>, call, message: AdminShell<'static>| {
            let world = &*ctx.modules;

            Ok(
                shell(world, &message.public_ip, &message.lan_ip, &message.user)
                    .await?
                    .map(|computer| {
                        ctx.trees.objects(call.tree).insert(AccessHandle {
                            access: Access {
                                computer,
                                user: message.user.to_string(),
                                ftp: false,
                                prev_ip: String::new(),
                                bounces: 0,
                            },
                            kind: AccessKind::Shell,
                        })
                    }),
            )
        },
    );
    routes.wait::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    _message: greyhack_world_programs::protocol::AdminSudo| {
            let cycles = ctx.cycles();
            let program = call.program;

            CycleAccess::change_dynamic(
                cycles,
                program,
                false,
                async move |ctx: &greyhack_world_router::RouteContext<'_, W>, call| {
                    let session = ctx
                        .trees
                        .get_mut(call.tree)
                        .expect("the indexed tree exists")
                        .session(program)
                        .expect("dispatch checked the active program");
                    let session = &ctx.trees.objects(call.tree).session(session)?;
                    let root = Access {
                        computer: session.computer,
                        user: ROOT_USER.to_owned(),
                        ftp: false,
                        prev_ip: session.prev_ip.clone(),
                        bounces: session.bounces,
                    };
                    let shell = ctx.trees.objects(call.tree).insert(AccessHandle {
                        access: root,
                        kind: AccessKind::Shell,
                    });

                    ctx.trees
                        .get_mut(call.tree)
                        .expect("the indexed tree exists")
                        .set_session(program, shell);

                    Ok(shell)
                },
            )
            .await
        },
    );
}

/// Finds the computer at the supplied address only if it contains `user`.
async fn shell(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    public_ip: &str,
    lan_ip: &str,
    user: &str,
) -> super::Result<Option<ComputerId>> {
    let Some(id) = world
        .module::<greyhack_world_core::CoreModule>()
        .networks
        .routers
        .computer_at(public_ip, lan_ip)
        .await?
    else {
        return Ok(None);
    };
    let has_user = world
        .module::<greyhack_world_core::CoreModule>()
        .computers
        .users
        .user(id, user)
        .await?
        .is_some();

    Ok(has_user.then_some(id))
}
