use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};
use greyhack_behaviour::is_valid_ip;

use super::access_or_crash;
use crate::error::IntrinsicError;
use crate::game::client::failure::smtp_text;
use crate::game::client::object;
use crate::game::client::server::{program, respond, server};
use crate::protocol::{CryptoSmtpUserList, SmtpUserList};

/// Returns users of the SMTP server at `ipAddress:port`.
/// Formats each entry as `name exists name@domain`.
#[native(method)]
pub fn smtp_user_list(
    vm: &mut Vm,
    this: &Value,
    #[name = "ipAddress"] address: &Value,
    port: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    if !object::game_map(vm, this) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let (Some(address), Some(port)) = (address.as_str(), port.as_int()) else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    if address.is_empty() || !is_valid_ip(address) {
        return Ok(NativeOutput::ready("Error: Invalid ip address"));
    }

    let Some(access) = access_or_crash(vm, this)? else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    let message = CryptoSmtpUserList {
        program: program(vm)?,
        crypto: access.reference(),
        address: address.into(),
        port: u16::try_from(port).unwrap_or(0),
    };

    Ok(respond(
        vm,
        server(vm)?.send_crypto_smtp_user_list(&message),
        |vm, users| match users {
            SmtpUserList::Users(users) => {
                let users = users.into_iter().map(Value::from).collect();

                Value::List(vm.heap_mut().add_list(users))
            }
            SmtpUserList::Failure(failure) => Value::from(smtp_text(&failure)),
        },
    ))
}
