use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};
use greyhack_behaviour::{Refusal, check_mail};

use super::account_after_cooldown;
use crate::error::IntrinsicError;
use crate::game::client::check::proceed;
use crate::game::client::object;
use crate::game::client::server::{program, respond, server};
use crate::protocol::{MailDraft, MetaMailSend, Refusal as Draft};

/// Sends `subject`/`message` from this account to `destAddress` (`MetaMailIntrinsics.send`).
#[native(method)]
pub fn send(
    vm: &mut Vm,
    this: &Value,
    #[name = "destAddress"] dest: &Value,
    subject: &Value,
    message: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    if !object::game_map(vm, this) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let (Some(dest), Some(subject), Some(message)) =
        (dest.as_str(), subject.as_str(), message.as_str())
    else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    let mail = proceed!(account_after_cooldown(vm, this));

    let draft = match check_mail(dest, subject, message) {
        Ok(()) => MailDraft::Email {
            to: dest.to_owned(),
            subject: subject.to_owned(),
            message: message.to_owned(),
        },
        Err(Refusal::InvalidMailAddress) => MailDraft::Refusal(Draft::InvalidMailAddress),
        Err(Refusal::MailSubjectTooLong) => MailDraft::Refusal(Draft::MailSubjectTooLong),
        Err(Refusal::MailMessageTooLong) => MailDraft::Refusal(Draft::MailMessageTooLong),
        Err(_) => MailDraft::Invalid,
    };

    if matches!(draft, MailDraft::Invalid) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let request = MetaMailSend {
        program: program(vm)?,
        mail,
        draft,
    };

    Ok(respond(
        vm,
        server(vm)?.send_meta_mail_send(&request),
        |_, outcome| outcome.response("send"),
    ))
}
