use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};
use greyhack_behaviour::NET_SPEEDS;

use super::public_ip;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminSetNetSpeed;

/// Sets this network's ISP speed pack (`NetworkLan.netSpeed`) as purchased through the ISP or company website.
/// Accepts 3, 6, 12, 20, or 25 and returns true; other speeds return false.
#[native(method)]
#[expect(
    clippy::float_cmp,
    reason = "NetworkLan.netSpeed accepts only the exact numeric values of its speed-pack enum"
)]
pub fn set_net_speed(
    vm: &mut Vm,
    this: &Value,
    speed: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "set_net_speed";

    let Some(public_ip) = public_ip(vm, this, NAME) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let Some(speed) = speed.as_number() else {
        return Err(IntrinsicError::InvalidArguments(NAME));
    };
    let Some(&net_speed) = NET_SPEEDS.iter().find(|pack| f64::from(**pack) == speed) else {
        return Ok(NativeOutput::ready(Value::from(false)));
    };
    let message = AdminSetNetSpeed {
        program: program(vm)?,
        public_ip: public_ip.into(),
        net_speed,
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_set_net_speed(&message),
        |_, set: bool| set,
    ))
}
