//! Reverse-shell clients (`MetaExploitIntrinsics.rshell_client`).

use greyhack_world_core::Access;
use greyhack_world_core::routes::require_connection;

use crate::routes::Result;
use crate::routes::protocol::{Failure, IntoProtocol as _, RshellClientOutcome};
use crate::{ComputerId, RshellClientFailure};

/// Starts a reverse-shell client connected to `address:port`.
pub(super) async fn rshell_client(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    access: &Access,
    terminal_computers: &[ComputerId],
    address: &str,
    port: u16,
    process_name: &str,
) -> Result<RshellClientOutcome> {
    require_connection(world, terminal_computers).await?;

    Ok(
        match world
            .module::<crate::LibraryModule>()
            .metaxploit
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
                RshellClientOutcome::Failure(Failure::Refusal(refusal.into_protocol()))
            }
            Some(Err(RshellClientFailure::Connection(failure))) => {
                RshellClientOutcome::Failure(Failure::Rshell(
                    crate::routes::protocol::rshell_failure(failure, address, port),
                ))
            }
        },
    )
}
