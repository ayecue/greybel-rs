use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::reference;
use crate::error::IntrinsicError;
use crate::game::client::failure::unit_test_text;
use crate::game::client::object;
use crate::game::client::server::{program, respond, server};
use crate::protocol::DebugLibraryUnitTesting;

#[native(method)]
pub fn unit_testing(
    vm: &mut Vm,
    this: &Value,
    error_lines: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    if !object::game_map(vm, this) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let Value::List(lines) = error_lines else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let Some(reference) = reference(vm, this) else {
        return Ok(NativeOutput::ready("error: file not found"));
    };
    let error_lines = lines
        .items(vm.heap())
        .iter()
        .map(|line| i64::from(line.as_int().unwrap_or(0)))
        .collect();
    let message = DebugLibraryUnitTesting {
        program: program(vm)?,
        library: reference.reference(),
        error_lines,
    };

    Ok(respond(
        vm,
        server(vm)?.send_debug_library_unit_testing(&message),
        |_, report| unit_test_text(&report),
    ))
}
