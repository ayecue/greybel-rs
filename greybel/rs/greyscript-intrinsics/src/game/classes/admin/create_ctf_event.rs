use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::is_admin;
use crate::error::IntrinsicError;
use crate::game::client::options::{options, string_option};
use crate::game::client::server::{program, respond, server};
use crate::protocol::{AdminCreateCtfEvent, CtfEventOptions};

#[native(method)]
pub fn create_ctf_event(
    vm: &mut Vm,
    this: &Value,
    #[name = "options"] options_value: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "create_ctf_event";

    if !is_admin(vm, this, NAME) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let map = options(options_value, NAME)?;
    let heap = vm.heap();
    let field = |key| string_option(heap, map, key, NAME);
    let event = CtfEventOptions {
        name: field("eventName")?.unwrap_or_default(),
        user: field("user")?.unwrap_or_default(),
        password: field("password")?.unwrap_or_default(),
        description: field("description")?.unwrap_or_default(),
        public_address: field("publicAddress")?.unwrap_or_default(),
        creator_name: field("creatorName")?.unwrap_or_default(),
        mail_content: field("mailContent")?.unwrap_or_default(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_create_ctf_event(&AdminCreateCtfEvent {
            program: program(vm)?,
            event,
        }),
        |_, created: bool| created,
    ))
}
