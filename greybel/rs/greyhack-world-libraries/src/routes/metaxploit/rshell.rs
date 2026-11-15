//! Reverse-shell clients (`MetaExploitIntrinsics.rshell_client`).

use greyhack_world_core::routes::require_connection;
use greyhack_world_core::{Access, ComputerId, ObjectTableExt as _};

use crate::objects::ObjectTableExt as _;
use crate::routes::Result;
use crate::routes::protocol::{
    Failure, IntoProtocol as _, ObjectId, Outcome, RshellClientOutcome, RshellRemoveSelection,
};
use crate::{RshellClient, RshellClientFailure, RshellRemoveFailure};

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

/// Removes reverse shells selected by `rshell_remove`.
pub(super) async fn rshell_remove(
    world: &impl greyhack_world_router::WorldContext<Computer = ComputerId>,
    objects: &greyhack_world_programs::ObjectTableHandle<ComputerId>,
    metaxploit: ObjectId,
    shells: RshellRemoveSelection,
    terminal_computers: &[ComputerId],
) -> Result<Outcome> {
    let access = objects.metaxploit(metaxploit)?.access;

    require_connection(world, terminal_computers).await?;

    let selection = match shells {
        RshellRemoveSelection::All => Ok(None),
        RshellRemoveSelection::InvalidType => Err(RshellRemoveFailure::ExpectedList),
        RshellRemoveSelection::InvalidItem => Err(RshellRemoveFailure::OnlyReverseShells),
        RshellRemoveSelection::Shells(ids) => {
            let clients = ids
                .into_iter()
                .map(|id| {
                    let shell = objects.reverse_shell(id)?;

                    Ok(RshellClient {
                        computer: shell.0.computer,
                        user: shell.0.user,
                        pid: shell.1,
                    })
                })
                .collect::<greyhack_world_core::routes::Result<Vec<_>>>()?;

            Ok(Some(clients))
        }
    };
    let result = world
        .module::<crate::LibraryModule>()
        .metaxploit
        .remove_rshell_clients(
            access.computer,
            selection
                .as_ref()
                .map(Option::as_deref)
                .map_err(|failure| *failure),
        )
        .await?;

    Ok(match result {
        None => Outcome::Null,
        Some(Ok(true)) => Outcome::True,
        Some(Ok(false)) => Outcome::False,
        Some(Err(failure)) => Outcome::Failure(Failure::RshellRemove(failure.into_protocol())),
    })
}
