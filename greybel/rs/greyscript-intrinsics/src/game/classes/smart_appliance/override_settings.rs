use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::appliance;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::SmartApplianceOverrideSettings;

#[native(method)]
pub fn override_settings(
    vm: &mut Vm,
    this: &Value,
    power: &Value,
    temperature: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let (Some(power), Some(temperature)) = (power.as_number(), temperature.as_number()) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let Some(appliance) = appliance(vm, this, "override_settings") else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = SmartApplianceOverrideSettings {
        program: program(vm)?,
        appliance,
        power,
        temperature,
    };

    Ok(respond(
        vm,
        server(vm)?.send_smart_appliance_override_settings(&message),
        |_, outcome| outcome.response("override_settings"),
    ))
}
