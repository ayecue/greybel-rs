use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use crate::error::IntrinsicError;
use crate::game::classes::ctf;
use crate::game::client::server::{program, respond, server};
use crate::protocol::PlayerGetCtf;

#[native]
pub fn get_ctf(
    vm: &mut Vm,
    user: &Value,
    password: &Value,
    #[name = "eventName"] event_name: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let (Some(event_name), Some(user), Some(password)) =
        (event_name.as_str(), user.as_str(), password.as_str())
    else {
        return Ok(NativeOutput::ready(
            "get_ctf: user and password cannot be null",
        ));
    };
    let message = PlayerGetCtf {
        program: program(vm)?,
        event: event_name.to_owned().into(),
        user: user.to_owned().into(),
        password: password.to_owned().into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_player_get_ctf(&message),
        |vm, ctf| match ctf {
            Some(ctf) => ctf::create(vm, ctf),
            None => Ok(Value::from("get_ctf: user or password incorrect")),
        },
    ))
}
