use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::public_ip;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminHireNpc;

/// Hires an NPC into a position vacated by death or firing, like the company hiring website (`NetworkLan.HireNpc`).
/// `jobRole` is Grey Hack's `JobRole` name, such as `"Technical_Support"`. Returns true or Grey Hack's failure reason.
#[native(method)]
pub fn hire_npc(
    vm: &mut Vm,
    this: &Value,
    #[name = "jobRole"] job_role: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "hire_npc";

    let Some(public_ip) = public_ip(vm, this, NAME) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let Some(job_role) = job_role.as_str() else {
        return Err(IntrinsicError::InvalidArguments(NAME));
    };
    let message = AdminHireNpc {
        program: program(vm)?,
        public_ip: public_ip.into(),
        job_role: job_role.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_hire_npc(&message),
        |_, outcome| outcome.response(NAME),
    ))
}
