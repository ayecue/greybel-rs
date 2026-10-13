//! Captured packets (`HelperScript.GetPendingSniffers`).

use greyhack_world::{SnifferPacket, World};

use crate::Result;
use crate::connection::objects::Access;
use crate::protocol::{FileFailure, SniffedPacket};

/// Describes a captured `packet` (`HelperScript.GetPendingSniffers`). When `encode_source` is set, saves
/// its encoder beside `program_path` as `user`.
pub(super) async fn capture(
    world: &mut World,
    access: &Access,
    packet: SnifferPacket,
    encode_source: bool,
    program_path: &str,
    user: &str,
) -> Result<SniffedPacket> {
    let encoder_failure = if encode_source {
        world
            .save_packet_encoder(access.computer, &packet, program_path, user)
            .await?
            .map(|failure| match failure {
                greyhack_world::EncoderFailure::NoPermission { path } => {
                    FileFailure::NoReadPermission(path)
                }
                greyhack_world::EncoderFailure::File(failure) => failure.into(),
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
