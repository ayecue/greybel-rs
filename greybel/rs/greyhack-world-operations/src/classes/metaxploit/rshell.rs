//! Reverse-shell clients (`MetaExploitIntrinsics.rshell_client`).

use greyhack_world::{ComputerId, RshellClientFailure, World};

use super::require_connection;
use crate::Result;
use crate::connection::objects::Access;
use crate::protocol::{Failure, RshellClientOutcome, RshellFailure};

/// Starts a reverse-shell client connected to `address:port`.
pub(super) async fn rshell_client(
    world: &mut World,
    access: &Access,
    terminal_computers: &[ComputerId],
    address: &str,
    port: u16,
    process_name: &str,
) -> Result<RshellClientOutcome> {
    require_connection(world, terminal_computers).await?;

    Ok(
        match world
            .start_reverse_shell(
                access.computer,
                &access.user,
                address,
                port,
                process_name,
                terminal_computers,
            )
            .await?
        {
            None => RshellClientOutcome::Null,
            Some(Ok(())) => RshellClientOutcome::True,
            Some(Err(RshellClientFailure::Refusal(refusal))) => {
                RshellClientOutcome::Failure(Failure::Refusal(refusal.into()))
            }
            Some(Err(RshellClientFailure::Connection(failure))) => RshellClientOutcome::Failure(
                Failure::Rshell(RshellFailure::of(failure, address, port)),
            ),
        },
    )
}
