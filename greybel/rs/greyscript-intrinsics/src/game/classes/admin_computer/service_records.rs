use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::address;
use crate::error::IntrinsicError;
use crate::game::client::object::ComputerAddress;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminServiceRecords;

/// Returns JSON records from this computer's criminals, employees, or students SQL `service` database.
/// Uses a game UI RPC, not a script intrinsic (`FileHelperServer.LeerFichaServerRpc`).
#[native(method)]
pub fn service_records(
    vm: &mut Vm,
    this: &Value,
    service: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "service_records";
    let Some(ComputerAddress { public_ip, lan_ip }) = address(vm, this, NAME) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let Some(service) = service.as_str() else {
        return Err(IntrinsicError::InvalidArguments(NAME));
    };
    let message = AdminServiceRecords {
        program: program(vm)?,
        public_ip: public_ip.into(),
        lan_ip: lan_ip.into(),
        service: service.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_service_records(&message),
        |_, records: Option<String>| records,
    ))
}
