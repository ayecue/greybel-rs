use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::address;
use crate::error::IntrinsicError;
use crate::game::client::failure::failure_value;
use crate::game::client::object::ComputerAddress;
use crate::game::client::server::{program, respond, server};
use crate::protocol::{AdminAccessCctv, CctvAccess};

/// Opens this camera's browser channel using `password`.
/// Returns null when a password prompt is required, otherwise returns camera data.
#[native(method)]
pub fn access_cctv(
    vm: &mut Vm,
    this: &Value,
    password: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "access_cctv";
    let Some(ComputerAddress { public_ip, lan_ip }) = address(vm, this, NAME) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let password = if password.is_null() {
        None
    } else {
        Some(
            password
                .as_str()
                .ok_or(IntrinsicError::InvalidArguments(NAME))?
                .to_owned(),
        )
    };
    let message = AdminAccessCctv {
        program: program(vm)?,
        public_ip: public_ip.into(),
        lan_ip: lan_ip.into(),
        password: password.map(Into::into),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_access_cctv(&message),
        |_, access: CctvAccess| match access {
            CctvAccess::Prompt => Ok(Value::Null),
            CctvAccess::Data(data) => Ok(Value::from(data)),
            CctvAccess::Failure(failure) => {
                failure_value("access_cctv", &failure)
            }
        },
    ))
}
