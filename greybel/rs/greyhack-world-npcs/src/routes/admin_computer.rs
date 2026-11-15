//! NPC operations of the single-computer `admin.so` object.

use greyhack_world_programs::protocol::{AdminAddNpc, AdminNpcLogin};
use greyhack_world_router::RouteTable;

use crate::routes::Result;

pub(crate) fn register<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    message: AdminAddNpc<'static>| {
            add_npc(
                &*ctx.modules,
                &message.public_ip,
                &message.lan_ip,
                &message.user,
                message.delay,
                &message.name,
            )
            .await
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    message: AdminNpcLogin<'static>| {
            // Apply social engineering (`Persona.ProcesaIngenieraSocial`): the NPC logs in through SSH/FTP,
            // exposing credentials to sniffers along the path.
            let Some(computer) = ctx
                .modules
                .module::<greyhack_world_core::CoreModule>()
                .networks
                .routers
                .computer_at(&message.address, &message.lan_ip)
                .await?
            else {
                return Ok(false);
            };

            ctx.modules
                .module::<crate::NpcModule>()
                .schedules
                .npc_social_login(computer, &message.user)
                .await?;

            Ok(true)
        },
    );
}

/// Creates an offline NPC scheduled to log in after `delay` seconds and returns its ID.
async fn add_npc(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    public_ip: &str,
    lan_ip: &str,
    user: &str,
    delay: u64,
    name: &str,
) -> Result<Option<u32>> {
    let next_action_at = world
        .module::<greyhack_world_core::CoreModule>()
        .world
        .clone()
        .time_seconds()
        .saturating_add(delay);
    let Some(computer) = world
        .module::<greyhack_world_core::CoreModule>()
        .networks
        .routers
        .computer_at(public_ip, lan_ip)
        .await?
    else {
        return Ok(None);
    };

    Ok(world
        .module::<crate::NpcModule>()
        .npcs
        .add_npc(computer, user, name, next_action_at)
        .await?
        .map(|id| id.number()))
}
