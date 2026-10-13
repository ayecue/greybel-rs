use greybel_interpreter::{NativeFuture, Value, Vm, native};

use super::{MailTarget, mail_target};
use crate::error::IntrinsicError;
use crate::game::client::check::proceed;
use crate::game::client::server::{program, respond, server};
use crate::protocol::MetaMailRead;

/// Returns full messages for IDs in list `mailIDs` from a freshly read inbox.
/// Missing IDs produce `Mail not found` (`MetaMailIntrinsics.read`).
#[native(method)]
pub fn read(
    vm: &mut Vm,
    this: &Value,
    #[name = "mailIDs"] ids: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let MailTarget { mail, mail_ids } = proceed!(mail_target(vm, this, ids));

    let message = MetaMailRead {
        program: program(vm)?,
        mail,
        mail_ids: mail_ids.map(Into::into),
    };

    Ok(respond(
        vm,
        server(vm)?.send_meta_mail_read(&message),
        |vm, reads| reads.response(vm, "read"),
    ))
}
