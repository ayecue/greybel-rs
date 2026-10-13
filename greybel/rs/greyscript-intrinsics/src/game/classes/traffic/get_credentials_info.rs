use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::device;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::TrafficNetGetCredentialsInfo;

#[native(method)]
pub fn get_credentials_info(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(traffic) = device(vm, this, "get_credentials_info") else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = TrafficNetGetCredentialsInfo {
        program: program(vm)?,
        traffic,
    };

    Ok(respond(
        vm,
        server(vm)?.send_traffic_net_get_credentials_info(&message),
        |_, info| info.response("get_credentials_info"),
    ))
}
