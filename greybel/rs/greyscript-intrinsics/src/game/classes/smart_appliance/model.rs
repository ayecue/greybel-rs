use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::appliance;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::SmartApplianceModel;

#[native(method)]
pub fn model(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(appliance) = appliance(vm, this, "model") else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = SmartApplianceModel {
        program: program(vm)?,
        appliance,
    };

    Ok(respond(
        vm,
        server(vm)?.send_smart_appliance_model(&message),
        |_, model| model.response("model"),
    ))
}
