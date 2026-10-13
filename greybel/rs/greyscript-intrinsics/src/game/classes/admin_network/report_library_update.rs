use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::public_ip;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminReportLibraryUpdate;

/// Reports the newest library file `library` as this network's update, as its administrator replying to the
/// update-libraries mail does (`Persona.UpdateReportedNetworks`). The network's older library files of that name take
/// it when the game restores them (`FileSystem.ApplyReportedLibUpdates`), and its computers made after it start with
/// it. Returns true, or false when the world has no library of that name.
#[native(method)]
pub fn report_library_update(
    vm: &mut Vm,
    this: &Value,
    library: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "report_library_update";

    let Some(public_ip) = public_ip(vm, this, NAME) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let Some(library) = library.as_str() else {
        return Err(IntrinsicError::InvalidArguments(NAME));
    };
    let message = AdminReportLibraryUpdate {
        program: program(vm)?,
        public_ip: public_ip.into(),
        library: library.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_report_library_update(&message),
        |_, outcome| outcome.response(NAME),
    ))
}
