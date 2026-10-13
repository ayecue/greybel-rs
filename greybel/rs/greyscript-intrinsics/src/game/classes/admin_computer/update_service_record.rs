use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::address;
use crate::error::IntrinsicError;
use crate::game::client::object::ComputerAddress;
use crate::game::client::server::{program, respond, server};
use crate::protocol::{AdminUpdateServiceRecord, Outcome};

/// Replaces the same person's record in this computer's SQL `service` database with JSON `record`.
/// Returns empty on success or the game UI error (`FileHelperServer.UpdateFichaServerRpc`).
#[native(method)]
pub fn update_service_record(
    vm: &mut Vm,
    this: &Value,
    service: &Value,
    record: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "update_service_record";
    let Some(ComputerAddress { public_ip, lan_ip }) = address(vm, this, NAME) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let (Some(service), Some(record)) = (service.as_str(), record.as_str()) else {
        return Err(IntrinsicError::InvalidArguments(NAME));
    };
    let message = AdminUpdateServiceRecord {
        program: program(vm)?,
        public_ip: public_ip.into(),
        lan_ip: lan_ip.into(),
        service: service.into(),
        record: record.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_update_service_record(&message),
        |_, outcome: Outcome| outcome.into_value_or_empty("update_service_record"),
    ))
}
