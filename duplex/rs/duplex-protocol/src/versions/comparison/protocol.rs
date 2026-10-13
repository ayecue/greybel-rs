use super::{Comparison, Mode};
use crate::error::DefinitionError;
use crate::model::{Response, ResponseBody, Role, Span, Stream};

impl<'a> Comparison<'a> {
    pub(super) fn compare(&mut self) -> Result<(), DefinitionError> {
        let (old_definition, new_definition) = (self.older, self.newer);

        if let (Some(old), Some(new)) = (old_definition.core, new_definition.core)
            && old.version.major != new.version.major
        {
            return Err(DefinitionError::new(
                new.span,
                format!(
                    "uses the core {}, of another major than the previous version's {}",
                    new.version, old.version
                ),
            ));
        }

        match (&old_definition.protocol, &new_definition.protocol) {
            (None, None) => {}
            (Some(old), Some(new)) => {
                if old.channel != new.channel {
                    return Err(DefinitionError::new(
                        new.span,
                        format!("changes the channel from {:?}", old.channel),
                    ));
                }

                let payloads = [
                    ("open", &old.handshake.open, &new.handshake.open),
                    ("ack", &old.handshake.ack, &new.handshake.ack),
                    ("refuse", &old.handshake.refusal, &new.handshake.refusal),
                ];

                for (keyword, old_payload, new_payload) in payloads {
                    match (old_payload, new_payload) {
                        (None, None) => {}
                        (Some(old_payload), Some(new_payload)) => {
                            let what = format!("the `{keyword}` payload");

                            self.ty(old_payload, new_payload, Mode::Exact, new.span, &what)?;
                        }
                        _ => {
                            return Err(DefinitionError::new(
                                new.span,
                                format!("adds or removes the handshake's `{keyword}` payload"),
                            ));
                        }
                    }
                }

                for role in Role::ALL {
                    self.messages(role)?;
                }
            }
            (_, new) => {
                let span = new.as_ref().map_or_else(Span::default, |new| new.span);

                return Err(DefinitionError::new(span, "adds or removes the protocol"));
            }
        }

        self.by_name()
    }

    pub(super) fn response(
        &mut self,
        old: &Response,
        new: &Response,
        span: Span,
        name: &str,
    ) -> Result<(), DefinitionError> {
        let what = format!("the response of `{name}`");

        match (&old.body, &new.body) {
            (ResponseBody::Value(old), ResponseBody::Value(new)) => {
                self.ty(old, new, Mode::Append, span, &what)?;
            }
            (ResponseBody::Stream(old), ResponseBody::Stream(new)) => {
                self.stream(old, new, span, &what)?;
            }
            _ => {
                return Err(DefinitionError::new(
                    span,
                    format!("changes whether `{name}` streams its response"),
                ));
            }
        }

        match (&old.failure, &new.failure) {
            (None, None) => Ok(()),
            (Some(old), Some(new)) => {
                let what = format!("what `{name}` fails with");

                self.ty(old, new, Mode::Append, span, &what)
            }
            _ => Err(DefinitionError::new(
                span,
                format!("changes whether `{name}` declares a failure"),
            )),
        }
    }

    pub(super) fn stream(
        &mut self,
        old: &Stream,
        new: &Stream,
        span: Span,
        what: &str,
    ) -> Result<(), DefinitionError> {
        for (part, verb, old, new) in [
            ("start", "starts", &old.start, &new.start),
            ("end", "ends", &old.end, &new.end),
        ] {
            match (old, new) {
                (None, None) => {}
                (Some(old), Some(new)) => {
                    self.ty(
                        old,
                        new,
                        Mode::Append,
                        span,
                        &format!("the {part} of {what}"),
                    )?;
                }
                _ => {
                    return Err(DefinitionError::new(
                        span,
                        format!("changes whether {what} {verb} with a value"),
                    ));
                }
            }
        }

        self.ty(
            &old.item,
            &new.item,
            Mode::Append,
            span,
            &format!("the items of {what}"),
        )
    }

    pub(super) fn messages(&mut self, role: Role) -> Result<(), DefinitionError> {
        let (old_definition, new_definition) = (self.older, self.newer);
        let (Some(old), Some(new)) = (&old_definition.protocol, &new_definition.protocol) else {
            return Ok(());
        };
        let old_messages: Vec<_> = old.inbound_messages(role).collect();
        let new_messages: Vec<_> = new.inbound_messages(role).collect();

        if let Some(old_extra) = old_messages.get(new_messages.len()) {
            return Err(DefinitionError::new(
                new.span,
                format!("removes the message `{}`", old_extra.name),
            ));
        }

        for (old_message, new_message) in old_messages.iter().zip(&new_messages) {
            let span = new_message.span;
            let name = &new_message.name;

            match (&old_message.stream, &new_message.stream) {
                (None, None) => {}
                (Some(old), Some(new)) => {
                    self.stream(
                        &old.stream,
                        &new.stream,
                        span,
                        &format!("the stream of `{name}`"),
                    )?;
                }
                _ => {
                    return Err(DefinitionError::new(
                        span,
                        format!("changes whether `{name}` streams"),
                    ));
                }
            }

            match (&old_message.response, &new_message.response) {
                (None, None) => {}
                (Some(old), Some(new)) => self.response(old, new, span, name)?,
                _ => {
                    return Err(DefinitionError::new(
                        span,
                        format!("changes whether `{name}` has a response"),
                    ));
                }
            }

            self.fields(&old_message.fields, &new_message.fields, Mode::Append, span)?;

            let since = self.old_additions.message(old_message);

            self.additions.set_message(role, new_message.id, since);
        }

        for new_extra in new_messages.iter().skip(old_messages.len()) {
            self.additions.set_message(role, new_extra.id, self.minor);
        }

        Ok(())
    }
}
