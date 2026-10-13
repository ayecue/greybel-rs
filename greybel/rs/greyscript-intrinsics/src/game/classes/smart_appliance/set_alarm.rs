use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::appliance;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::SmartApplianceSetAlarm;

#[native(method)]
pub fn set_alarm(
    vm: &mut Vm,
    this: &Value,
    enable: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let Some(enable) = enable.as_number() else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let Some(appliance) = appliance(vm, this, "set_alarm") else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = SmartApplianceSetAlarm {
        program: program(vm)?,
        appliance,
        enable: enable != 0.0,
    };

    Ok(respond(
        vm,
        server(vm)?.send_smart_appliance_set_alarm(&message),
        |_, outcome| outcome.response("set_alarm"),
    ))
}
