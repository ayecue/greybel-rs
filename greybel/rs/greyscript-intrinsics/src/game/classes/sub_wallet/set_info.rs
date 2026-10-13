use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::sub_wallet;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::SubWalletSetInfo;

#[native(method)]
pub fn set_info(vm: &mut Vm, this: &Value, info: &Value) -> Result<NativeFuture, IntrinsicError> {
    let (Some(info), Some(sub_wallet)) = (info.as_str(), sub_wallet(vm, this)) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = SubWalletSetInfo {
        program: program(vm)?,
        sub_wallet,
        info: info.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_sub_wallet_set_info(&message),
        |_, result| result,
    ))
}
