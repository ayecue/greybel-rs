use greybel_interpreter::{NativeFuture, NativeOutput, Text, Value, Vm, native};
use greyhack_behaviour::MAIL_PREVIEW as PREVIEW;

use super::account_after_cooldown;
use crate::error::IntrinsicError;
use crate::game::client::check::proceed;
use crate::game::client::failure::failure_value;
use crate::game::client::object;
use crate::game::client::server::{program, respond, server};
use crate::protocol::{InboxContent, MetaMailFetch};

/// Returns Grey Hack's `MailID: <id> From: <sender> Subject: <subject>` inbox summaries.
/// Truncates each body to its first [`PREVIEW`] string positions.
#[native(method)]
pub fn fetch(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    if !object::game_map(vm, this) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let mail = proceed!(account_after_cooldown(vm, this));

    let message = MetaMailFetch {
        program: program(vm)?,
        mail,
    };

    Ok(respond(
        vm,
        server(vm)?.send_meta_mail_fetch(&message),
        |vm, content| match content {
            InboxContent::Failure(failure) => failure_value("fetch", &failure),
            InboxContent::Emails(emails) => {
                let heap = vm.heap();
                let summaries = emails
                    .into_iter()
                    .map(|email| {
                        let body = Text::from(email.body.as_str());
                        let units = heap.text_units(&body);
                        let preview = if units.len() > PREVIEW {
                            format!("{}...", &*units.slice(0..PREVIEW))
                        } else {
                            email.body.clone()
                        };

                        Value::from(format!(
                            "\n\nMailID: {}\nFrom: {}\nSubject: {}\n{}",
                            email.id,
                            email.from,
                            email.subject,
                            preview.replace("\n\n", "\n")
                        ))
                    })
                    .collect();

                Ok(Value::List(vm.heap_mut().add_list(summaries)))
            }
        },
    ))
}
