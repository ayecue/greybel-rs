//! `metaMail` inbox and sending intrinsics (`MetaMailIntrinsics`), one file per method.

use greybel_interpreter::{Native, Value, Vm};
use greyhack_behaviour::META_MAIL_COOLDOWN;

use crate::error::IntrinsicError;
use crate::game::client::object::{GameObject, Kind};
use crate::game::client::{object, stopwatch};
use crate::game::game;
use crate::protocol::ObjectId;

mod delete_all;
mod delete_mail;
mod fetch;
mod read;
mod send;
mod target;

pub(crate) const METHODS: &[(&str, &Native)] = &[
    ("fetch", &fetch::fetch::NATIVE),
    ("read", &read::read::NATIVE),
    ("send", &send::send::NATIVE),
    ("delete", &delete_mail::delete_mail::NATIVE),
    ("delete_all", &delete_all::delete_all::NATIVE),
];

pub(super) use target::{MailTarget, account_after_cooldown, mail_target};

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
    if stopwatch::elapsed_ms::<MetaMailWatch>(vm)
        .is_some_and(|elapsed| elapsed < META_MAIL_COOLDOWN.as_millis())
    {
        return Ok(Some(Value::from(cooldown_text())));
    }

    stopwatch::restart::<MetaMailWatch>(vm);

    let now = vm.clock().now();

    Ok((!game(vm)?.server.use_mail_cooldown(now)).then(|| Value::from(cooldown_text())))
}

/// Constructs the mail account object for server-issued `id`.
pub(crate) fn create(vm: &mut Vm, id: ObjectId) -> Result<Value, IntrinsicError> {
    object::create_of(vm, Kind::MetaMail, GameObject::MetaMail, id)
}
