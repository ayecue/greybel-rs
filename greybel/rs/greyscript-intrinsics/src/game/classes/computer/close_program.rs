use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::computer;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::{ComputerCloseProgram, ProgramClose};

#[native(method)]
pub fn close_program(
    vm: &mut Vm,
    this: &Value,
    #[name = "PID"] pid: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let Some(pid) = pid.as_int() else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let Some(access) = computer(vm, this, "close_program") else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = ComputerCloseProgram {
        program: program(vm)?,
        computer: access.reference(),
        pid,
    };

    Ok(respond(
        vm,
        server(vm)?.send_computer_close_program(&message),
        |vm, result| match result {
            ProgramClose::Outcome(outcome) => outcome.response("close_program"),
            ProgramClose::OwnProcess => {
                vm.exit();

                Ok(Value::TRUE)
            }
        },
    ))
}
