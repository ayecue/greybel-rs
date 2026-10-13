use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::sub_wallet;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::game::send_output;
use crate::protocol::{Outcome, SubWalletMining};

/// Mines this coin into the sub-wallet from the script's computer.
#[native(method)]
pub fn mining(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(sub_wallet) = sub_wallet(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = SubWalletMining {
        program: program(vm)?,
        sub_wallet,
    };

    Ok(respond(
        vm,
        server(vm)?.send_sub_wallet_mining(&message),
        |vm, outcome| {
            // Grey Hack flushes output when the transfer starts; the transfer returns true.
            if matches!(outcome, Outcome::True) {
                send_output(vm);
            }

            outcome.response("mining")
        },
    ))
}
