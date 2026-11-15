use std::ops::ControlFlow;

use greybel_interpreter::{Value, Vm};

use super::cooldown;
use crate::error::IntrinsicError;
use crate::game::client::check::{Proceed, stop};
use crate::game::client::receiver::receiver_or_crash;
use crate::game::client::{object, receiver};
use crate::protocol::ObjectId;

/// The result of parsing the mail ID argument.
enum MailIds {
    /// The argument is not a list.
    NotList,
    /// A list entry is not a string.
    InvalidId,
    /// Every list entry is a mail ID string.
    Valid(Vec<String>),
}

/// Parses mail IDs from list `ids`; invalid IDs are detected after re-reading the account.
fn mail_ids(vm: &Vm, ids: &Value) -> MailIds {
    let Value::List(list) = ids else {
        return MailIds::NotList;
    };

    list.items(vm.heap())
        .iter()
        .map(|id| id.as_str().map(str::to_owned))
        .collect::<Option<Vec<_>>>()
        .map_or(MailIds::InvalidId, MailIds::Valid)
}

/// Applies the mail cooldown, then returns the account helper, matching Grey Hack's dereference order.
pub(in crate::game::classes) fn account_after_cooldown(
    vm: &mut Vm,
    this: &Value,
) -> Result<Proceed<ObjectId>, IntrinsicError> {
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
pub(in crate::game::classes) struct MailTarget {
    pub(super) mail: ObjectId,
    pub(super) mail_ids: Option<Vec<String>>,
}

/// Runs the checks `MetaMailIntrinsics.read` and `delete_mail` share, in the game's order.
pub(in crate::game::classes) fn mail_target(
    vm: &mut Vm,
    this: &Value,
    ids: &Value,
) -> Result<Proceed<MailTarget>, IntrinsicError> {
    if !object::game_map(vm, this) {
        return Ok(stop(Value::Null));
    }

    let mail_ids = match mail_ids(vm, ids) {
        MailIds::NotList => return Ok(stop(Value::Null)),
        MailIds::InvalidId => None,
        MailIds::Valid(ids) => Some(ids),
    };

    Ok(match account_after_cooldown(vm, this)? {
        ControlFlow::Continue(mail) => ControlFlow::Continue(MailTarget { mail, mail_ids }),
        ControlFlow::Break(output) => ControlFlow::Break(output),
    })
}
