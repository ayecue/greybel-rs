use greybel_interpreter::{NativeFuture, NativeOutput, PrintOptions, Value, Vm, native};

use super::session;
use crate::error::IntrinsicError;
use crate::game::client::server::{operate, program, server};
use crate::protocol::{FloodResult, FloodWarning, NetSessionFloodConnection};

/// Floods the session computer (`NetSessionIntrinsics.flood_connection`).
/// Returns null after it reboots; while it remains online, the program waits until stopped.
#[native(method)]
pub fn flood_connection(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "flood_connection";

    let Some(target) = session(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = NetSessionFloodConnection {
        program: program(vm)?,
        net_session: target.reference(),
    };
    let request = server(vm)?.send_net_session_flood_connection(&message);
    let stdout = vm.stdout().clone();

    Ok(Box::pin(async move {
        match operate(NAME, request).await {
            Ok(FloodResult::Active { address, warning }) => {
                stdout.print(&start_text(&address, &warning), PrintOptions::default());

                std::future::pending().await
            }
            Ok(FloodResult::Reboot { address, warning }) => {
                stdout.print(&start_text(&address, &warning), PrintOptions::default());
                stdout.print("remote connection interrupted", PrintOptions::default());

                NativeOutput::new(Value::Null)
            }
            Ok(FloodResult::Gone) => NativeOutput::new(Value::Null),
            Err(error) => NativeOutput::new(Err::<Value, _>(error)),
        }
    }))
}

/// Text printed when a flood of `address` starts, with the `warning` the game shows.
fn start_text(address: &str, warning: &FloodWarning) -> String {
    let warning = match warning {
        FloodWarning::None => "",
        FloodWarning::SuperAdmin => {
            "\nWarning: Super administrator detected. Additional DDoS traffic required."
        }
        FloodWarning::CrownShield => "\nWarning: DDoS traffic blocked by CrownShield.",
    };

    format!("Flooding {address}...\nPress Ctrl+C to stop{warning}")
}

#[cfg(test)]
#[path = "tests/flood_connection.rs"]
mod tests;
