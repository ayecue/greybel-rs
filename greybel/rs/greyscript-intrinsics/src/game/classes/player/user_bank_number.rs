use greybel_interpreter::{NativeFuture, Vm, native};

use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::PlayerUserBankNumber;

#[native]
pub fn user_bank_number(vm: &mut Vm) -> Result<NativeFuture, IntrinsicError> {
    let message = PlayerUserBankNumber {
        program: program(vm)?,
    };

    Ok(respond(
        vm,
        server(vm)?.send_player_user_bank_number(&message),
        |_, number| number,
    ))
}
