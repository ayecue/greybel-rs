use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::{alarm_map, public_ip};
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminPhoneAlarm;

/// Reads an NPC smartphone's alarm after finalizing any completed lateness update.
#[native(method)]
pub fn phone_alarm(
    vm: &mut Vm,
    this: &Value,
    user: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "phone_alarm";

    let Some(public_ip) = public_ip(vm, this, NAME) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let Some(user) = user.as_str() else {
        return Err(IntrinsicError::InvalidArguments(NAME));
    };
    let message = AdminPhoneAlarm {
        program: program(vm)?,
        public_ip: public_ip.into(),
        user: user.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_phone_alarm(&message),
        alarm_map,
    ))
}
