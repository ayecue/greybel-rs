use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::computer;
use crate::error::IntrinsicError;
use crate::game::client::login_encryption::LoginEncryption;
use crate::game::client::server::{program, respond, server};
use crate::protocol::ComputerShowProcs;

/// Returns processes in `ps` format, with one `dsession` per logged-in user first (`ComputerIntrinsics` `show_procs`).
#[native(method)]
pub fn show_procs(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(access) = computer(vm, this, "show_procs") else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    LoginEncryption::deny_in(vm, "show_procs")?;

    let message = ComputerShowProcs {
        program: program(vm)?,
        computer: access.reference(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_computer_show_procs(&message),
        |_, processes| {
            processes.map(|processes| {
                let mut out = PROCESS_HEADER.to_owned();

                for process in processes {
                    out.push_str(&format!(
                        "\n{} {} {}% {}% {}",
                        process.user,
                        process.pid,
                        one_decimal(process.cpu),
                        one_decimal(process.memory),
                        process.name
                    ));
                }

                out
            })
        },
    ))
}

const PROCESS_HEADER: &str = "USER PID CPU MEM COMMAND";

/// Formats like `value.ToString("N1")`: one decimal place and thousands separators.
fn one_decimal(value: f32) -> String {
    mono_number::format_f32(value, "N1").expect("N1 applies to floats")
}

#[cfg(test)]
#[path = "tests/show_procs.rs"]
mod tests;
