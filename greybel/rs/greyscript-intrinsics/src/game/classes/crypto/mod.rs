//! `crypto` password, SMTP, and Wi-Fi capture/cracking intrinsics (`CryptoIntrinsics`), one file per method.

use std::ops::ControlFlow;

use greybel_interpreter::{Native, Value, Vm};
use greyhack_behaviour::check_password;

use crate::error::IntrinsicError;
use crate::game::client::{object, receiver};
use crate::game::client::receiver::{receiver, receiver_or_crash};
use crate::game::client::check::{Proceed, stop};
use crate::game::client::server::refuse;
use crate::runtime::Handle;

mod aircrack;
mod aireplay;
mod airmon;
mod decipher;
mod decrypt;
mod encrypt;
mod is_encrypted;
mod smtp_user_list;

pub(crate) const METHODS: &[(&str, &Native)] = &[
    ("decipher", &decipher::decipher::NATIVE),
    ("encrypt", &encrypt::encrypt::NATIVE),
    ("decrypt", &decrypt::decrypt::NATIVE),
    ("is_encrypted", &is_encrypted::is_encrypted::NATIVE),
    ("smtp_user_list", &smtp_user_list::smtp_user_list::NATIVE),
    ("airmon", &airmon::airmon::NATIVE),
    ("aireplay", &aireplay::aireplay::NATIVE),
    ("aircrack", &aircrack::aircrack::NATIVE),
];

/// Returns the crypto library's host computer accessed as the session user.
pub(crate) fn access(vm: &Vm, this: &Value) -> Option<Handle> {
    receiver::<receiver::Crypto>(vm, this).cloned()
}

/// Returns the crypto helper, crashing where Grey Hack dereferences it without a null check.
fn access_or_crash(vm: &Vm, this: &Value) -> Result<Option<Handle>, IntrinsicError> {
    Ok(receiver_or_crash::<receiver::Crypto>(vm, this)?.cloned())
}

/// Arguments `CryptoIntrinsics.encrypt` and `decrypt` pass to the server once their checks pass.
struct FileKey {
    access: Handle,
    path: String,
    password: String,
}

/// Runs the checks `CryptoIntrinsics.encrypt` and `decrypt` share, in the game's order.
fn file_key(
    vm: &Vm,
    this: &Value,
    intrinsic: &'static str,
    path: &Value,
    password: &Value,
) -> Result<Proceed<FileKey>, IntrinsicError> {
    if !object::game_map(vm, this) {
        return Ok(stop(Value::Null));
    }

    let (Some(path), Some(password)) = (path.as_str(), password.as_str()) else {
        return Ok(stop(Value::Null));
    };

    if let Err(refusal) = check_password(password) {
        return refuse(intrinsic, refusal).map(ControlFlow::Break);
    }

    let Some(access) = access_or_crash(vm, this)? else {
        return Ok(stop(Value::Null));
    };

    if path.is_empty() {
        return Ok(stop(format!("{intrinsic}: Invalid file path")));
    }

    Ok(ControlFlow::Continue(FileKey {
        access,
        path: path.into(),
        password: password.into(),
    }))
}
