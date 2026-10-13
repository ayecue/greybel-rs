use greybel_interpreter::{NativeFuture, Value, Vm, native};

use super::{MailTarget, mail_target};
use crate::error::IntrinsicError;
use crate::game::client::check::proceed;
use crate::game::client::server::{program, respond, server};
use crate::protocol::MetaMailDelete;

/// Deletes IDs in list `mailIDs` from a freshly read inbox.
/// Returns true if any requested mail existed, otherwise null (`MetaMailIntrinsics.delete_mail`).
#[native(method)]
pub fn delete_mail(
    vm: &mut Vm,
    this: &Value,
    #[name = "mailIDs"] ids: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let MailTarget { mail, mail_ids } = proceed!(mail_target(vm, this, ids));

    let message = MetaMailDelete {
        program: program(vm)?,
        mail,
        mail_ids: mail_ids.map(Into::into),
    };

    Ok(respond(
        vm,
        server(vm)?.send_meta_mail_delete(&message),
        |_, outcome| outcome.response("delete"),
    ))
}
