//! Process listing, process termination, and rebooting (`ComputerIntrinsics`).

use greyhack_world::{ComputerId, ProcessClose, World};

use crate::Result;
use crate::connection::objects::Access;
use crate::protocol::{Failure, Outcome, ProcessUsage, ProgramClose};

/// Returns `ps`-style process usage; NPC user processes are intentionally omitted.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn show_procs(world: &World, access: &Access) -> Result<Option<Vec<ProcessUsage>>> {
    Ok(world
        .process_usage(access.computer)
        .await?
        .map(|processes| {
            processes
                .into_iter()
                .map(|process| ProcessUsage {
                    user: process.user,
                    pid: process.pid,
                    cpu: process.cpu,
                    memory: process.memory,
                    name: process.name,
                })
                .collect()
        }))
}

/// Terminates `pid` with the object's permissions and closes service-daemon ports. The caller's own process, when it
/// is a command a terminal runs (`terminal_command`), stops the caller instead.
///
/// # Errors
///
/// Returns errors from database reads or writes.
pub async fn close_program(
    world: &mut World,
    access: &Access,
    pid: i32,
    script_process: Option<(ComputerId, u32)>,
    terminal_command: Option<(ComputerId, u32)>,
) -> Result<ProgramClose> {
    Ok(
        match world
            .close_process(
                access.computer,
                &access.user,
                pid,
                script_process,
                terminal_command,
            )
            .await?
        {
            ProcessClose::Gone => ProgramClose::Outcome(Outcome::Null),
            ProcessClose::Missing => ProgramClose::Outcome(Outcome::False),
            ProcessClose::Success => ProgramClose::Outcome(Outcome::True),
            ProcessClose::OwnProcess => ProgramClose::OwnProcess,
            ProcessClose::Refusal(failure) => {
                ProgramClose::Outcome(Outcome::Failure(Failure::from(failure)))
            }
        },
    )
}

/// Requires root or root-group access. Safe mode is restricted to the player's computer.
///
/// # Errors
///
/// Returns errors from database reads or writes.
pub async fn reboot(
    world: &mut World,
    access: &Access,
    terminal_computers: &[ComputerId],
    safe_mode: bool,
) -> Result<Outcome> {
    Ok(
        match world
            .reboot_as(access.computer, &access.user, terminal_computers, safe_mode)
            .await?
        {
            Ok(true) => Outcome::True,
            Ok(false) => Outcome::Null,
            Err(failure) => Outcome::Failure(failure.into()),
        },
    )
}
