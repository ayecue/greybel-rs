//! Packet sniffing through metaxploit (`MetaExploitIntrinsics.sniffer`).

use greyhack_world_core::{Access, ObjectTableExt as _};
use greyhack_world_router::protocol::ScriptFailure;
use greyhack_world_router::{CycleAccess, RouteTable};

use crate::SnifferPacket;
use crate::routes::protocol::{FileFailure, IntoProtocol as _, MetaxploitSniffer, SniffedPacket};
use crate::routes::{OperationError, Result};

pub(crate) fn register<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.wait::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: MetaxploitSniffer<'static>| {
            let cycles = ctx.cycles();
            let program = call.program;
            let start = CycleAccess::change_dynamic(
                cycles,
                program,
                false,
                async move |ctx: &greyhack_world_router::RouteContext<'_, W>, call| {
                    let world = &*ctx.modules;
                    let intrusion = world.module::<crate::IntrusionModule>();
                    let access = intrusion
                        .library_objects
                        .metaxploit_access(&ctx.trees.objects(call.tree), message.metaxploit)?;
                    let sniffer = intrusion.sniffers.start_sniffer(access.computer).await?;

                    Ok::<_, OperationError>(sniffer.map(|sniffer| (access, sniffer)))
                },
            )
            .await?;
            let Some((access, work)) = start else {
                return Ok(None);
            };
            let packet =
                CycleAccess::defer_dynamic::<SnifferPacket, W, greyhack_world_core::WorldError>(
                    cycles, program, work,
                )
                .await
                .ok_or_else(|| OperationError::Script(ScriptFailure::ProgramEnded))?;

            CycleAccess::change_dynamic(
                cycles,
                program,
                false,
                async move |ctx: &greyhack_world_router::RouteContext<'_, W>, call| {
                    let world = &*ctx.modules;
                    let session = ctx
                        .trees
                        .get_mut(call.tree)
                        .expect("the indexed tree exists")
                        .session(program)
                        .expect("dispatch checked the active program");
                    let user = ctx.trees.objects(call.tree).session(session)?.user.clone();

                    capture(
                        world,
                        &access,
                        packet,
                        message.encode_source,
                        &message.program_path,
                        &user,
                    )
                    .await
                },
            )
            .await
            .map(Some)
        },
    );
}

/// Describes a captured `packet` (`HelperScript.GetPendingSniffers`). When `encode_source` is set, saves
/// its encoder beside `program_path` as `user`.
async fn capture(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    access: &Access,
    packet: SnifferPacket,
    encode_source: bool,
    program_path: &str,
    user: &str,
) -> Result<SniffedPacket> {
    let encoder_failure = if encode_source {
        world
            .module::<crate::IntrusionModule>()
            .sniffers
            .save_packet_encoder(access.computer, &packet, program_path, user)
            .await?
            .map(|failure| match failure {
                crate::services::sniffer::encoder::EncoderFailure::NoPermission { path } => {
                    FileFailure::NoReadPermission(path)
                }
                crate::services::sniffer::encoder::EncoderFailure::File(failure) => {
                    failure.into_protocol()
                }
            })
    } else {
        None
    };

    Ok(SniffedPacket {
        source: packet.source,
        destination: packet.destination,
        protocol: packet.protocol,
        user: packet.user,
        password: packet.password,
        encrypted: packet.encrypted,
        encoder_failure,
    })
}
