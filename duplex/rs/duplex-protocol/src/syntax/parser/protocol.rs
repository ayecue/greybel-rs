use super::{
    ACK, CLOSE_THRESHOLD, FLAVOUR, HEALTH_CHECK, INTERVAL, LEFT_OUT, LIMIT, MAX_MESSAGE, MESSAGE,
    OFF, OPEN, Parser, REFUSE, RENAMES, RESULT, SECURITY, SIGNED_KEYWORD, STREAM, STREAM_WINDOW,
    TIMEOUT, UNHEALTHY_THRESHOLD,
};
use crate::error::DefinitionError;
use crate::model::{
    DocumentationRef, Field, Flavour, Handshake, HealthCheckSetting, Message, Peer, Protocol,
    Response, ResponseBody, Role, Security, SecurityLevel, Span, Stream, StreamField, TagList,
    TypeRef,
};
use crate::syntax::lexer::TokenKind;

impl Parser {
    pub(super) fn protocol(
        &mut self,
        docs: DocumentationRef,
        tags: TagList,
        span: Span,
    ) -> Result<Protocol, DefinitionError> {
        let name = self.name()?;

        self.expect('=')?;

        let channel = self.text()?;

        self.expect('{')?;

        let mut peers: [Option<Peer>; 2] = [None, None];
        let mut handshake = Handshake::default();
        let mut flavours = Vec::new();
        let mut security = None;
        let mut limit = None;
        let mut messages = Vec::new();

        while !self.consume('}') {
            let docs = self.docs();
            let token = self.peek().clone();

            if self.consume_keyword(FLAVOUR) {
                flavours.push(self.flavour(token.span)?);
                continue;
            }

            if self.consume_keyword(SECURITY) {
                if security.is_some() {
                    return Err(DefinitionError::new(
                        token.span,
                        "the protocol declares its security once",
                    ));
                }

                security = Some(self.security()?);
                continue;
            }

            if self.consume_keyword(LIMIT) {
                if limit.is_some() {
                    return Err(DefinitionError::new(
                        token.span,
                        "the protocol declares its limit once",
                    ));
                }

                limit = Some(self.size()?);
                self.expect(';')?;
                continue;
            }

            if let Some(slot) = self.handshake_slot(&mut handshake) {
                if !docs.is_empty() {
                    return Err(DefinitionError::new(
                        token.span,
                        "a handshake's payload takes no doc comment; its type's does",
                    ));
                }

                if slot.is_some() {
                    let keyword = match &token.kind {
                        TokenKind::Ident(keyword) => keyword.as_str(),
                        _ => "",
                    };

                    return Err(DefinitionError::new(
                        token.span,
                        format!("the protocol declares one `{keyword}` payload"),
                    ));
                }

                self.bump();
                *slot = Some(self.type_ref()?);
                self.expect(';')?;
                continue;
            }

            let role = match &token.kind {
                TokenKind::Ident(keyword) => Role::from_keyword(keyword),
                _ => None,
            };
            let Some(role) = role else {
                return Err(self.token_error(
                    "`client`, `provider`, `open`, `ack`, `refuse`, `flavour`, `security`, `limit` or `}`",
                ));
            };

            self.bump();

            let slot = &mut peers[usize::from(role.id())];

            if slot.is_some() {
                return Err(DefinitionError::new(
                    token.span,
                    format!("the protocol has one {}", role.keyword()),
                ));
            }

            let name = self.name()?;

            *slot = Some(Peer {
                docs,
                name,
                role,
                span: token.span,
            });
            self.expect('{')?;

            let mut id = 0;

            while !self.consume('}') {
                messages.push(self.message(id, role)?);
                id += 1;
            }
        }

        let [Some(client), Some(provider)] = peers else {
            return Err(DefinitionError::new(
                span,
                "the protocol has a client and a provider",
            ));
        };

        Ok(Protocol {
            docs,
            tags,
            name,
            channel,
            handshake,
            flavours,
            security,
            limit,
            peers: [client, provider],
            messages,
            span,
        })
    }

    pub(super) fn security(&mut self) -> Result<Security, DefinitionError> {
        let span = self.peek().span;
        let keyword = self.name()?;
        let level = SecurityLevel::ALL
            .into_iter()
            .find(|level| level.keyword() == keyword)
            .ok_or_else(|| {
                DefinitionError::new(
                    span,
                    format!(
                        "a protocol's security is `encrypted` or `authenticated`, not `{keyword}`"
                    ),
                )
            })?;
        let signed = self.consume_keyword(SIGNED_KEYWORD);

        self.expect(';')?;

        Ok(Security { level, signed })
    }

    pub(super) fn flavour(&mut self, span: Span) -> Result<Flavour, DefinitionError> {
        let transport = self.name()?;

        self.expect('{')?;

        let mut health_check = None;
        let mut stream_window = None;
        let mut timeout = None;
        let mut max_message = None;

        while !self.consume('}') {
            let setting = self.peek().span;
            let name = self.name()?;
            let set = match name.as_str() {
                HEALTH_CHECK => health_check.is_some(),
                STREAM_WINDOW => stream_window.is_some(),
                TIMEOUT => timeout.is_some(),
                MAX_MESSAGE => max_message.is_some(),
                _ => {
                    return Err(DefinitionError::new(
                        setting,
                        format!(
                            "a flavour sets `{HEALTH_CHECK}`, `{STREAM_WINDOW}`, `{TIMEOUT}` or `{MAX_MESSAGE}`, not `{name}`"
                        ),
                    ));
                }
            };

            if set {
                return Err(DefinitionError::new(
                    setting,
                    format!("the flavour sets `{name}` once"),
                ));
            }

            self.expect('=')?;
            match name.as_str() {
                HEALTH_CHECK => health_check = Some(self.health_check()?),
                STREAM_WINDOW => {
                    let window = self.number()?;

                    if window == 0 {
                        return Err(DefinitionError::new(
                            setting,
                            "a stream's window fits an item at least",
                        ));
                    }

                    stream_window = Some(window);
                }
                TIMEOUT => timeout = Some(self.duration()?),
                _ => max_message = Some(self.size()?),
            }

            self.expect(';')?;
        }

        Ok(Flavour {
            transport,
            health_check,
            stream_window,
            timeout,
            max_message,
            span,
        })
    }

    pub(super) fn health_check(&mut self) -> Result<HealthCheckSetting, DefinitionError> {
        if self.consume_keyword(OFF) {
            return Ok(HealthCheckSetting::Off);
        }

        self.expect('{')?;

        let (mut interval, mut unhealthy_threshold, mut close_threshold) = (None, None, None);

        while !self.consume('}') {
            let span = self.peek().span;
            let name = self.name()?;
            let slot = match name.as_str() {
                INTERVAL => &mut interval,
                UNHEALTHY_THRESHOLD => &mut unhealthy_threshold,
                CLOSE_THRESHOLD => &mut close_threshold,
                _ => {
                    return Err(DefinitionError::new(
                        span,
                        format!(
                            "a health check sets `{INTERVAL}`, `{UNHEALTHY_THRESHOLD}` or `{CLOSE_THRESHOLD}`, not `{name}`"
                        ),
                    ));
                }
            };

            if slot.is_some() {
                return Err(DefinitionError::new(
                    span,
                    format!("the health check sets `{name}` once"),
                ));
            }

            self.expect('=')?;
            *slot = Some(if name == INTERVAL {
                self.duration()?
            } else {
                self.number()?
            });
            if !self.consume(',') {
                self.expect('}')?;
                break;
            }
        }

        Ok(HealthCheckSetting::On {
            interval,
            unhealthy_threshold,
            close_threshold,
        })
    }

    pub(super) fn handshake_slot<'h>(
        &self,
        handshake: &'h mut Handshake,
    ) -> Option<&'h mut Option<TypeRef>> {
        let TokenKind::Ident(keyword) = &self.peek().kind else {
            return None;
        };

        match keyword.as_str() {
            OPEN => Some(&mut handshake.open),
            ACK => Some(&mut handshake.ack),
            REFUSE => Some(&mut handshake.refusal),
            _ => None,
        }
    }

    pub(super) fn message(&mut self, id: u32, receiver: Role) -> Result<Message, DefinitionError> {
        let docs = self.docs();
        let tags = self.tags()?;
        let token = self.peek().clone();

        if !self.consume_keyword(MESSAGE) {
            return Err(self.token_error("`message` or `}`"));
        }

        let name = self.name()?;
        let renames = if self.consume_keyword(RENAMES) {
            Some(self.name()?)
        } else {
            None
        };
        let (fields, stream) = self.message_fields()?;
        let response = if self.peek().kind == TokenKind::Arrow {
            self.bump();

            Some(self.response()?)
        } else {
            None
        };

        if stream.is_some() && response.is_none() {
            return Err(DefinitionError::new(
                token.span,
                format!(
                    "`{name}` streams, so it declares a response: what its receiver makes of it"
                ),
            ));
        }

        let (mut timeout, mut limit) = (None, None);

        loop {
            let span = self.peek().span;

            if self.consume_keyword(TIMEOUT) {
                if response.is_none() {
                    return Err(DefinitionError::new(
                        span,
                        format!("`{name}` is a notification: no response to time out"),
                    ));
                }

                if timeout.replace(self.duration()?).is_some() {
                    return Err(DefinitionError::new(
                        span,
                        format!("`{name}` has one timeout"),
                    ));
                }
            } else if self.consume_keyword(LIMIT) {
                if limit.replace(self.size()?).is_some() {
                    return Err(DefinitionError::new(
                        span,
                        format!("`{name}` has one limit"),
                    ));
                }
            } else {
                break;
            }
        }

        Ok(Message {
            docs,
            tags,
            name,
            renames,
            id,
            receiver,
            fields,
            stream,
            response,
            timeout,
            limit,
            span: token.span,
        })
    }

    pub(super) fn message_fields(
        &mut self,
    ) -> Result<(Vec<Field>, Option<StreamField>), DefinitionError> {
        let mut stream = None;
        let fields = self.field_list(|parser, docs, name, span| {
            if stream.is_some() {
                return Err(DefinitionError::new(
                    span,
                    "a stream is a message's last field",
                ));
            }

            let stream_span = parser.peek().span;
            let parsed_stream = parser.stream()?;

            if parsed_stream.start.is_some() {
                return Err(DefinitionError::new(
                    stream_span,
                    "a message's stream starts with no value: its other fields are its start",
                ));
            }

            stream = Some(StreamField {
                docs,
                name,
                stream: parsed_stream,
                span,
            });

            Ok(())
        })?;

        Ok((fields, stream))
    }

    pub(super) fn response(&mut self) -> Result<Response, DefinitionError> {
        if !self.consume_keyword(RESULT) {
            return Ok(Response {
                body: self.response_body()?,
                failure: None,
            });
        }

        self.expect('<')?;

        let body = self.response_body()?;

        self.expect(',')?;

        let failure = self.type_ref()?;

        self.expect('>')?;

        Ok(Response {
            body,
            failure: Some(failure),
        })
    }

    pub(super) fn response_body(&mut self) -> Result<ResponseBody, DefinitionError> {
        if self.is_keyword(STREAM) {
            return Ok(ResponseBody::Stream(self.stream()?));
        }

        Ok(ResponseBody::Value(self.type_ref()?))
    }

    pub(super) fn stream(&mut self) -> Result<Stream, DefinitionError> {
        if !self.consume_keyword(STREAM) {
            return Err(self.token_error("`Stream`"));
        }

        let (start, end) = if self.consume('(') {
            let start = self.left_out_or_type()?;

            self.expect(',')?;

            let end = self.left_out_or_type()?;

            self.expect(')')?;

            (start, end)
        } else {
            (None, None)
        };
        let item = self.argument()?;
        let items = self.length_requirement(Self::count_bound)?;

        Ok(Stream {
            start,
            end,
            item,
            items,
        })
    }

    pub(super) fn left_out_or_type(&mut self) -> Result<Option<TypeRef>, DefinitionError> {
        if self.consume_keyword(LEFT_OUT) {
            return Ok(None);
        }

        self.type_ref().map(Some)
    }
}
