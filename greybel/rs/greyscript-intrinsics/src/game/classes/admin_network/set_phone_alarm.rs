use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::{alarm_map, public_ip};
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminSetPhoneAlarm;

/// Changes an NPC smartphone alarm like the phone clock program.
#[native(method)]
pub fn set_phone_alarm(
    vm: &mut Vm,
    this: &Value,
    user: &Value,
    hour: &Value,
    minute: &Value,
    #[name = "deleteAlarm"] delete_alarm: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "set_phone_alarm";

    let Some(public_ip) = public_ip(vm, this, NAME) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let (Some(user), Some(hour), Some(minute), Some(delete)) = (
        user.as_str(),
        hour.as_int(),
        minute.as_int(),
        delete_alarm.as_number(),
    ) else {
        return Err(IntrinsicError::InvalidArguments(NAME));
    };
    let message = AdminSetPhoneAlarm {
        program: program(vm)?,
        public_ip: public_ip.into(),
        user: user.into(),
        hour: i64::from(hour),
        minute: i64::from(minute),
        delete: delete != 0.0,
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_set_phone_alarm(&message),
        alarm_map,
    ))
}
