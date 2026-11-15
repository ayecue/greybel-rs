use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};
use greyhack_behaviour::check_ctf_password;

use crate::error::IntrinsicError;
use crate::game::client::server::{program, refuse, respond, server};
use crate::protocol::PlayerResetCtfPassword;

#[native]
pub fn reset_ctf_password(
    vm: &mut Vm,
    #[name = "new_password"] new_password: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let Some(password) = new_password.as_str() else {
        return Ok(NativeOutput::ready(
            "reset_ctf_password: password cannot be null",
        ));
    };

    if let Err(refusal) = check_ctf_password(password) {
        return refuse("reset_ctf_password", &refusal);
    }

    let message = PlayerResetCtfPassword {
        program: program(vm)?,
        password: password.to_owned().into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_player_reset_ctf_password(&message),
        |_, outcome| outcome.response("reset_ctf_password"),
    ))
}
