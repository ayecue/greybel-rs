//! `metaMail` inbox and sending intrinsics (`MetaMailIntrinsics`), one file per method.

use std::ops::ControlFlow;

use greybel_interpreter::{Native, Value, Vm};
use greyhack_behaviour::META_MAIL_COOLDOWN;

use crate::error::IntrinsicError;
use crate::game::client::check::{Proceed, stop};
use crate::game::client::object::{GameObject, Kind};
use crate::game::client::receiver::receiver_or_crash;
use crate::game::client::{object, receiver, stopwatch};
use crate::game::game;
use crate::protocol::ObjectId;

mod delete_all;
mod delete_mail;
mod fetch;
mod read;
mod send;

pub(crate) const METHODS: &[(&str, &Native)] = &[
    ("fetch", &fetch::fetch::NATIVE),
    ("read", &read::read::NATIVE),
    ("send", &send::send::NATIVE),
    ("delete", &delete_mail::delete_mail::NATIVE),
    ("delete_all", &delete_all::delete_all::NATIVE),
];

/// Returns the text produced by every MetaMail method during cooldown (`HelperScript.UseMetaMailCooldownOrError`).
fn cooldown_text() -> String {
    format!(
        "MetaMail: Wait {} seconds before using mail again.",
        META_MAIL_COOLDOWN.as_secs()
    )
}

/// Program MetaMail stopwatch (`HelperScript.metaMailCooldownWatch`).
struct MetaMailWatch;

/// Applies the program MetaMail cooldown, then the player's (`HelperScript.UseMetaMailCooldownOrError`).
/// Returns the common method response while either cooldown is active, otherwise none.
fn cooldown(vm: &mut Vm) -> Result<Option<Value>, IntrinsicError> {
    if stopwatch::elapsed_ms::<MetaMailWatch>(vm).is_some_and(|elapsed| elapsed < META_MAIL_COOLDOWN.as_millis()) {
        return Ok(Some(Value::from(cooldown_text())));
    }

    stopwatch::restart::<MetaMailWatch>(vm);

    let now = vm.clock().now();

    Ok((!game(vm)?.server.use_mail_cooldown(now)).then(|| Value::from(cooldown_text())))
}

/// Parses mail IDs from list `ids`.
/// Returns `None` for a non-list and `Some(None)` for a non-string ID; methods detect the latter only after re-reading the account.
fn mail_ids(vm: &Vm, ids: &Value) -> Option<Option<Vec<String>>> {
    let Value::List(list) = ids else {
        return None;
    };

    Some(
        list.items(vm.heap())
            .iter()
            .map(|id| id.as_str().map(str::to_owned))
            .collect(),
    )
}

/// Constructs the mail account object for server-issued `id`.
pub(crate) fn create(vm: &mut Vm, id: ObjectId) -> Result<Value, IntrinsicError> {
    object::create_of(vm, Kind::MetaMail, GameObject::MetaMail, id)
}

/// Applies the mail cooldown, then returns the account helper, matching Grey Hack's dereference order.
fn account_after_cooldown(vm: &mut Vm, this: &Value) -> Result<Proceed<ObjectId>, IntrinsicError> {
    if let Some(refusal) = cooldown(vm)? {
        return Ok(stop(refusal));
    }

    Ok(
        match receiver_or_crash::<receiver::MetaMail>(vm, this)?.map(|mail| mail.reference()) {
            Some(mail) => ControlFlow::Continue(mail),
            None => stop(Value::Null),
        },
    )
}

/// Mail IDs of `read` and `delete`, with the account they apply to.
struct MailTarget {
    mail: ObjectId,
    mail_ids: Option<Vec<String>>,
}

/// Runs the checks `MetaMailIntrinsics.read` and `delete_mail` share, in the game's order.
fn mail_target(vm: &mut Vm, this: &Value, ids: &Value) -> Result<Proceed<MailTarget>, IntrinsicError> {
    if !object::game_map(vm, this) {
        return Ok(stop(Value::Null));
    }

    let Some(mail_ids) = mail_ids(vm, ids) else {
        return Ok(stop(Value::Null));
    };

    Ok(match account_after_cooldown(vm, this)? {
        ControlFlow::Continue(mail) => ControlFlow::Continue(MailTarget { mail, mail_ids }),
        ControlFlow::Break(output) => ControlFlow::Break(output),
    })
}
