use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::is_admin;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminAcceptMission;

/// Accepts a procedural mission of the hack shop or police network at `shop_ip` like its website does and mails the
/// contract to the player; not available in Grey Hack. Returns the public address of the first network of the
/// mission, or Grey Hack's failure reason.
#[native(method)]
pub fn accept_mission(
    vm: &mut Vm,
    this: &Value,
    shop_ip: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "accept_mission";

    if !is_admin(vm, this, NAME) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let Some(shop_ip) = shop_ip.as_str() else {
        return Err(IntrinsicError::InvalidArguments(NAME));
    };
    let message = AdminAcceptMission {
        program: program(vm)?,
        shop_ip: shop_ip.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_accept_mission(&message),
        |_, acceptance| acceptance.response(NAME),
    ))
}
