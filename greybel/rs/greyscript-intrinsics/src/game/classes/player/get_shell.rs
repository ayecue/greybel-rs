use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use crate::error::IntrinsicError;
use crate::game::client::failure::inactive_account;
use crate::game::client::login_encryption::LoginEncryption;
use crate::game::client::object;
use crate::game::client::object::Access;
use crate::game::client::server::{program, respond, server};
use crate::game::game;
use crate::protocol::{PlayerGetShell, ShellLogin};

/// Returns a shell on the session computer.
/// Without credentials, uses the session user; otherwise uses `user` when `password` authenticates them.
#[native]
pub fn get_shell(
    vm: &mut Vm,
    #[default("")] user: &Value,
    #[default("")] password: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    LoginEncryption::deny_in(vm, "get_shell")?;

    let (Some(user), Some(password)) = (user.as_str(), password.as_str()) else {
        return Err(IntrinsicError::InvalidArguments("get_shell"));
    };

    if user.is_empty() && password.is_empty() {
        let shell = game(vm)?.session.shell.clone();

        return Ok(NativeOutput::ready(object::create_shell(vm, shell)));
    }

    let message = PlayerGetShell {
        program: program(vm)?,
        user: user.to_owned().into(),
        password: password.to_owned().into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_player_get_shell(&message),
        |vm, login| match login {
            ShellLogin::Shell(shell) => {
                let shell = Access::new(object::handle(vm, shell)?);

                Ok::<_, IntrinsicError>(object::create_shell(vm, shell))
            }
            ShellLogin::Null => Ok(Value::Null),
            ShellLogin::AccountInactive => Ok(Value::from(inactive_account())),
        },
    ))
}
