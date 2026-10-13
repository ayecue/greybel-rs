use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::account_after_cooldown;
use crate::error::IntrinsicError;
use crate::game::client::check::proceed;
use crate::game::client::object;
use crate::game::client::server::{program, respond, server};
use crate::protocol::MetaMailDeleteAll;

/// Deletes every inbox message (`MetaMailIntrinsics.delete_all`).
#[native(method)]
pub fn delete_all(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    if !object::game_map(vm, this) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let mail = proceed!(account_after_cooldown(vm, this));

    let message = MetaMailDeleteAll {
        program: program(vm)?,
        mail,
    };

    Ok(respond(
        vm,
        server(vm)?.send_meta_mail_delete_all(&message),
        |_, outcome| outcome.response("delete_all"),
    ))
}
