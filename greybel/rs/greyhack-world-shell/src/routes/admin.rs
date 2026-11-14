//! Shell-related global `admin.so` operations.

use greyhack_behaviour::ROOT_USER;
use greyhack_world_core::{Access, AccessHandle, AccessKind, ObjectTableExt as _};
use greyhack_world_router::{CycleAccess, RouteTable};

use crate::ShellContext;

pub(crate) fn register<W: ShellContext + 'static>(routes: &RouteTable) {
    routes.wait::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    _message: greyhack_world_router::protocol::AdminSudo| {
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
