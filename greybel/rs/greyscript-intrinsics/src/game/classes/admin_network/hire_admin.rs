use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::public_ip;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminHireRentalAdmin;

/// Hires an administrator for this rented-server network like the bank website (`BankHelperServer.HireAdminRentalServerRpc`).
/// Selects a person from a new network who intercepts all attacks while alive. Returns true or Grey Hack's failure reason.
#[native(method)]
pub fn hire_admin(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "hire_admin";

    let Some(public_ip) = public_ip(vm, this, NAME) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = AdminHireRentalAdmin {
        program: program(vm)?,
        public_ip: public_ip.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_hire_rental_admin(&message),
        |_, outcome| outcome.response(NAME),
    ))
}
