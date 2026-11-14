//! `start_terminal` launches a shell terminal as a program child, like `launch`.
//! Runs the host computer's `/bin/bash` or Grey Hack's built-in bash as the shell user from their home folder (`ShellIntrinsics.start_terminal`).

use greybel_interpreter::{ArgContainer, Native, NativeFuture, NativeOutput, Value, Vm};

use super::launch::{ChildLocation, ChildStart};
use super::shell;
use crate::game::client::failure::failure_text;
use crate::game::client::login_encryption::LoginEncryption;
use crate::protocol::{Failure, LaunchOutcome, ShellFailure, ShellStartTerminal};

pub static START_TERMINAL: Native = Native::from_async_fn(None, &[], true, start_terminal);

fn start_terminal(vm: &mut Vm, args: &ArgContainer) -> NativeFuture {
    let Some(access) = shell(vm, args.self_value(), "start_terminal") else {
        return NativeOutput::ready(Value::Null);
    };

    if let Err(error) = LoginEncryption::deny_in(vm, "start_terminal") {
        return NativeOutput::ready(Err::<Value, _>(error));
    }

    let Some((child, server)) = ChildStart::begin(vm) else {
        return NativeOutput::ready(false);
    };
    let request = server.send_shell_start_terminal(&ShellStartTerminal {
        program: child.game.program,
        shell: access.reference(),
    });

    Box::pin(async move {
        let outcome = match request {
            Ok(response) => response.await,
            Err(_) => return NativeOutput::new(false),
        };

        match outcome {
            Ok(LaunchOutcome::Program {
                session,
                source,
                source_map,
            }) => {
                // The terminal replaces the program (`HelperScript.AddPendingVM`): it inherits both program paths, starts in the user's home folder, and receives a new custom object.
                let paths = ChildLocation {
                    current: session.home.clone(),
                    launch: child.game.session.program_path.clone(),
                    program: child.game.session.program_path.clone(),
                };
                let child = ChildStart {
                    resets_custom_object: true,
                    terminal: true,
                    ..child
                };

                child
                    .enter(session, paths, source, source_map, Vec::new())
                    .await
            }
            // Returns the unavailable-computer result; other refusals are printed by the terminal.
            Ok(LaunchOutcome::Failure(
                failure @ Failure::Shell(ShellFailure::ComputerUnavailable),
            )) => match failure_text("start_terminal", &failure) {
                Ok(text) => NativeOutput::new(text),
                Err(error) => NativeOutput::new(Err::<Value, _>(error)),
            },
            Ok(LaunchOutcome::Failure(failure)) => match failure_text("start_terminal", &failure) {
                Ok(text) => NativeOutput::new(text),
                Err(error) => NativeOutput::new(Err::<Value, _>(error)),
            },
            Ok(LaunchOutcome::Refusal) | Err(_) => NativeOutput::new(false),
        }
    })
}
