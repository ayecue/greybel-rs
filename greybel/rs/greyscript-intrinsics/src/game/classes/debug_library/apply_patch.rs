use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};
use greyhack_behaviour::path;

use super::reference;
use crate::error::IntrinsicError;
use crate::game::client::object;
use crate::game::client::server::{program, respond, server};
use crate::game::game;
use crate::protocol::DebugLibraryApplyPatch;

#[native(method)]
pub fn apply_patch(
    vm: &mut Vm,
    this: &Value,
    #[name = "path"] path_value: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    if !object::game_map(vm, this) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let Some(path_value) = path_value.as_str() else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let Some(reference) = reference(vm, this) else {
        return Ok(NativeOutput::ready("error: file not found"));
    };
    let patch_path = path::resolve(path_value, &game(vm)?.session.current_path);
    let message = DebugLibraryApplyPatch {
        program: program(vm)?,
        library: reference.reference(),
        path: patch_path.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_debug_library_apply_patch(&message),
        |_, success| {
            if success {
                "The patch has been applied correctly."
            } else {
                ""
            }
        },
    ))
}
