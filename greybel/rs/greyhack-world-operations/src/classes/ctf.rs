//! Operations on CTF events (`CTFIntrinsics`).

use std::rc::Rc;

use duplex_core::{Channel, Driver};
use greyhack_world::{CtfEvent, World};

use crate::Result;
use crate::connection::Connection;
use crate::connection::objects::CtfRef;
use crate::protocol::{
    CtfGetCreatorName, CtfGetDescription, CtfGetMailContent, CtfGetTemplate, CtfPlayerSuccess,
    GameServer,
};

pub(super) fn attach<D: Driver>(connection: &Rc<Connection>, channel: &Channel<GameServer, D>) {
    connection.read(
        channel,
        async |world, objects, message: CtfGetDescription, _| {
            event_text(world, objects.ctf(message.ctf)?, |event| &event.description).await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: CtfGetTemplate, _| {
            event_text(world, objects.ctf(message.ctf)?, |event| {
                &event.public_address
            })
            .await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: CtfPlayerSuccess, _| {
            let ctf = objects.ctf(message.ctf)?;

            Ok(world.player_success(&ctf.event, ctf.computer).await?)
        },
    );
    connection.read(
        channel,
        async |world, objects, message: CtfGetCreatorName, _| {
            event_text(world, objects.ctf(message.ctf)?, |event| {
                &event.creator_name
            })
            .await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: CtfGetMailContent, _| {
            event_text(world, objects.ctf(message.ctf)?, |event| {
                &event.mail_content
            })
            .await
        },
    );
}

/// Extracts event text with `read` if the event still exists.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn event_text(
    world: &World,
    ctf: &CtfRef,
    read: impl FnOnce(&CtfEvent) -> &str,
) -> Result<Option<String>> {
    Ok(world
        .ctf_event(&ctf.event)
        .await?
        .map(|event| read(&event).to_owned()))
}
