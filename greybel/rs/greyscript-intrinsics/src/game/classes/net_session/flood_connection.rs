use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::session;
use crate::error::IntrinsicError;
use crate::game::client::server::{operate, program, server};
use crate::protocol::{FloodResult, NetSessionFloodConnection};

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

    Ok(Box::pin(async move {
        match operate(NAME, request).await {
            Ok(FloodResult::Active) => std::future::pending().await,
            Ok(FloodResult::Reboot) => NativeOutput::new(Value::Null),
            Ok(FloodResult::Gone) => NativeOutput::new(Value::Null),
            Err(error) => NativeOutput::new(Err::<Value, _>(error)),
        }
    }))
}
