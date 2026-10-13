use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::address;
use crate::error::IntrinsicError;
use crate::game::client::object::ComputerAddress;
use crate::game::client::options::{option, options, string_option};
use crate::game::client::server::{program, respond, server};
use crate::protocol::{AdminAddSmartAppliance, SmartApplianceOptions};

/// Converts this computer into a smart appliance for sandbox scenarios.
#[native(method)]
pub fn add_smart_appliance(
    vm: &mut Vm,
    this: &Value,
    #[name = "options"] options_value: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "add_smart_appliance";

    let Some(ComputerAddress { public_ip, lan_ip }) = address(vm, this, NAME) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let map = options(options_value, NAME)?;
    let heap = vm.heap();
    let model = string_option(heap, map, "model", NAME)?.unwrap_or_else(|| "FR/unknown".to_owned());
    let power = option(heap, map, "power")
        .and_then(Value::as_int)
        .unwrap_or(600);
    let temperature = option(heap, map, "temperature")
        .and_then(Value::as_int)
        .unwrap_or(20);
    let override_enabled =
        option(heap, map, "overrideEnabled").is_none_or(|value| heap.is_truthy(value));
    let message = AdminAddSmartAppliance {
        program: program(vm)?,
        public_ip: public_ip.into(),
        lan_ip: lan_ip.into(),
        appliance: SmartApplianceOptions {
            model,
            power,
            temperature,
            override_enabled,
        },
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_add_smart_appliance(&message),
        |_, added: bool| added,
    ))
}
