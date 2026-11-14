use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};
use greyhack_behaviour::{check_process_name, is_valid_ip};

use super::metaxploit_or_crash;
use crate::error::IntrinsicError;
use crate::game::client::failure::failure_value;
use crate::game::client::object;
use crate::game::client::server::{program, raise, refuse, server, settle};
use crate::protocol::{MetaxploitRshellClient, RshellClientOutcome};

#[native(method)]
pub fn rshell_client(
    vm: &mut Vm,
    this: &Value,
    address: &Value,
    #[default(1222)] port: &Value,
    #[name = "procName"]
    #[default("rshell_client")]
    process_name: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    if !object::game_map(vm, this) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let Some(address) = address.as_str() else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let Some(process_name) = process_name.as_str() else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let Some(port) = port.as_int() else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    if !is_valid_ip(address) {
        return Ok(NativeOutput::ready("rshell_client: Invalid IP address"));
    }

    if let Err(refusal) = check_process_name(process_name) {
        return refuse("rshell_client", refusal);
    }

    let Some(metaxploit) = metaxploit_or_crash(vm, this)? else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    let message = MetaxploitRshellClient {
        program: program(vm)?,
        metaxploit,
        address: address.into(),
        port: u16::try_from(port).unwrap_or(0),
        process_name: process_name.into(),
    };

    Ok(settle(
        server(vm)?.send_metaxploit_rshell_client(&message),
        move |_, outcome| {
            let value = match outcome.map_err(|failure| raise("rshell_client", failure))? {
                RshellClientOutcome::Null => Value::Null,
                RshellClientOutcome::True => Value::from(true),
                RshellClientOutcome::Failure(failure) => failure_value("rshell_client", &failure)?,
            };

            Ok::<_, IntrinsicError>(value)
        },
    ))
}
