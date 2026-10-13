//! Implements `launch` and child programs entered by it and `start_terminal`.
//! These async natives are hand-written because the native macro supplies no `vm` parameter.
//! After the server starts a child, the native enters a child context in the same VM and heap and returns when it exits.
//! Child-retained values, including closures in the custom object, remain usable by the parent; compiled code is cached across launches.

mod cache;
mod child_end;
mod child_start;

pub(crate) use cache::{CachedCode, LaunchCache, launch_key};
pub(super) use child_start::{ChildLocation, ChildStart};
use greybel_interpreter::{
    ArgContainer, Native, NativeDefault, NativeFuture, NativeOutput, NativeParam, Value, Vm,
};

use super::shell;
use crate::error::IntrinsicError;
use crate::game::client::failure::invalid_shell;
use crate::game::client::login_encryption::LoginEncryption;
use crate::game::client::object;
use crate::protocol::{LaunchOutcome, ShellLaunch};

static LAUNCH_PARAMS: &[NativeParam] = &[
    NativeParam::new("path"),
    NativeParam::with_default("params", NativeDefault::String("")),
];

/// Implements `launch(path, params)`: runs executable `path` in a child context and splits `params` on whitespace.
pub static LAUNCH: Native = Native::from_async_fn(None, LAUNCH_PARAMS, true, launch);

fn launch(vm: &mut Vm, args: &ArgContainer) -> NativeFuture {
    // `launch` requires a shell, a path, and text parameters (`ShellIntrinsics`).
    let invalid =
        || NativeOutput::ready(Err::<Value, _>(IntrinsicError::InvalidArguments("launch")));
    let path = args.get("path").as_str().filter(|path| !path.is_empty());
    let Some(path) = path.filter(|_| {
        object::game_map(vm, args.self_value()) && args.get("params").as_str().is_some()
    }) else {
        return invalid();
    };

    if let Err(error) = LoginEncryption::deny_in(vm, "launch") {
        return NativeOutput::ready(Err::<Value, _>(error));
    }

    let Some(access) = shell(vm, args.self_value(), "launch") else {
        return NativeOutput::ready(invalid_shell());
    };

    let path = path.to_owned();
    let params: Vec<String> = args
        .get("params")
        .as_str()
        .map(|params| params.split_whitespace().map(str::to_owned).collect())
        .unwrap_or_default();
    let Some((child, server)) = ChildStart::begin(vm) else {
        return NativeOutput::ready(false);
    };
    let request = server.send_shell_launch(&ShellLaunch {
        program: child.game.program,
        shell: access.reference(),
        path: path.clone().into(),
        params: params.clone().into(),
        current: child.game.session.current_path.clone().into(),
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
                let paths = ChildLocation {
                    current: session.home.clone(),
                    launch: child.game.session.program_path.clone(),
                    program: session.path.clone(),
                };

                child
                    .enter(session, paths, source, source_map, params)
                    .await
            }
            // The terminal prints the launch refusal.
            Ok(LaunchOutcome::Failure(failure)) => child.print_failure("launch", &failure),
            Ok(LaunchOutcome::Refusal) | Err(_) => NativeOutput::new(false),
        }
    })
}
