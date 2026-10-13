use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::reference;
use crate::error::IntrinsicError;
use crate::game::client::failure::scan_text;
use crate::game::client::object;
use crate::game::client::server::{program, respond, server};
use crate::protocol::DebugLibraryScanDebuglib;

#[native(method)]
pub fn scan_debuglib(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    if !object::game_map(vm, this) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let Some(reference) = reference(vm, this) else {
        return Ok(NativeOutput::ready("error: file not found"));
    };
    let message = DebugLibraryScanDebuglib {
        program: program(vm)?,
        library: reference.reference(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_debug_library_scan_debuglib(&message),
        |_, scan| scan_text(&scan),
    ))
}
