//! Operations on CTF events (`CTFIntrinsics`).

use greyhack_world_router::RouteTable;
use greyhack_world_router::protocol::{
    CtfGetCreatorName, CtfGetDescription, CtfGetMailContent, CtfGetTemplate, CtfPlayerSuccess,
};

use crate::routes::Result;
use crate::{CtfEvent, CtfRef, ObjectTableExt as _, PlayerContext};

pub(crate) fn register<W: PlayerContext + 'static>(routes: &RouteTable) {
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: CtfGetDescription| {
            let world = &*ctx.modules;

            event_text(
                world,
                &ctx.trees.objects(call.tree).ctf(message.ctf)?,
                |event| &event.description,
            )
            .await
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: CtfGetTemplate| {
            let world = &*ctx.modules;

            event_text(
                world,
                &ctx.trees.objects(call.tree).ctf(message.ctf)?,
                |event| &event.public_address,
            )
            .await
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: CtfPlayerSuccess| {
            let world = &*ctx.modules;
            let ctf = &ctx.trees.objects(call.tree).ctf(message.ctf)?;

            Ok(world
                .player()
                .ctf
                .player_success(&ctf.event, ctf.computer)
                .await?)
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: CtfGetCreatorName| {
            let world = &*ctx.modules;

            event_text(
                world,
                &ctx.trees.objects(call.tree).ctf(message.ctf)?,
                |event| &event.creator_name,
            )
            .await
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: CtfGetMailContent| {
            let world = &*ctx.modules;

            event_text(
                world,
                &ctx.trees.objects(call.tree).ctf(message.ctf)?,
                |event| &event.mail_content,
            )
            .await
        },
    );
}

/// Extracts event text with `read` if the event still exists.
///
/// # Errors
///
/// Returns errors from database reads.
async fn event_text(
    world: &impl PlayerContext,
    ctf: &CtfRef,
    read: impl FnOnce(&CtfEvent) -> &str,
) -> Result<Option<String>> {
    Ok(world
        .player()
        .ctf
        .ctf_event(&ctf.event)
        .await?
        .map(|event| read(&event).to_owned()))
}
