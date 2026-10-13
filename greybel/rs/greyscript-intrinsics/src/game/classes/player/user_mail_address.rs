use greybel_interpreter::{NativeFuture, Vm, native};

use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::PlayerUserMailAddress;

/// Returns the player's mail address when the script runs on their computer.
#[native]
pub fn user_mail_address(vm: &mut Vm) -> Result<NativeFuture, IntrinsicError> {
    let message = PlayerUserMailAddress {
        program: program(vm)?,
    };

    Ok(respond(
        vm,
        server(vm)?.send_player_user_mail_address(&message),
        |_, address| address,
    ))
}
