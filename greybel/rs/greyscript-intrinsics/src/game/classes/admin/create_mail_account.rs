use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::is_admin;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminCreateMailAccount;

/// Creates mail account `address` with `password`; not available in Grey Hack.
#[native(method)]
pub fn create_mail_account(
    vm: &mut Vm,
    this: &Value,
    address: &Value,
    password: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    if !is_admin(vm, this, "create_mail_account") {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let (Some(address), Some(password)) = (address.as_str(), password.as_str()) else {
        return Err(IntrinsicError::InvalidArguments("create_mail_account"));
    };
    let message = AdminCreateMailAccount {
        program: program(vm)?,
        address: address.into(),
        password: password.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_create_mail_account(&message),
        |_, created: bool| created,
    ))
}
