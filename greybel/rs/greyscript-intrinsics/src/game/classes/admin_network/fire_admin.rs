use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::public_ip;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminFireRentalAdmin;

/// Removes this network's hired administrator (`BankHelperServer.FireAdminRentalServerRpc`).
/// Returns true or Grey Hack's failure reason.
#[native(method)]
pub fn fire_admin(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "fire_admin";

    let Some(public_ip) = public_ip(vm, this, NAME) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = AdminFireRentalAdmin {
        program: program(vm)?,
        public_ip: public_ip.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_fire_rental_admin(&message),
        |_, outcome| outcome.response(NAME),
    ))
}
