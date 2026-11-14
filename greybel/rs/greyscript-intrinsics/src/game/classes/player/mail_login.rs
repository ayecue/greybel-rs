use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use crate::error::IntrinsicError;
use crate::game::classes::meta_mail;
use crate::game::client::failure::failure_value;
use crate::game::client::login_encryption::LoginEncryption;
use crate::game::client::server::{program, respond, server};
use crate::protocol::{ObjectOutcome, PlayerMailLogin};

/// Authenticates mail account `user` with `password` and returns a host-computer-bound `metaMail` (`mail_login`).
#[native]
pub fn mail_login(
    vm: &mut Vm,
    user: &Value,
    password: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let (Some(address), Some(password)) = (user.as_str(), password.as_str()) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let valid = address.split_once('@').is_some_and(|(name, domain)| {
        !name.is_empty() && !domain.is_empty() && !domain.contains('@')
    });

    if !valid {
        return Ok(NativeOutput::ready("mail_login: Invalid account name."));
    }

    LoginEncryption::deny_in(vm, "mail_login")?;

    let message = PlayerMailLogin {
        program: program(vm)?,
        address: address.to_owned().into(),
        password: password.to_owned().into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_player_mail_login(&message),
        |vm, outcome| match outcome {
            ObjectOutcome::Object(account) => meta_mail::create(vm, account),
            ObjectOutcome::Null => Ok(Value::Null),
            ObjectOutcome::False => Ok(Value::FALSE),
            ObjectOutcome::Failure(failure) => failure_value("mail_login", &failure),
        },
    ))
}
