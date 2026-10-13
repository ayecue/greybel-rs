use super::{Encoding, FieldsPlan, Source};
use crate::model::{
    Definition, Length, Message, Peer, Protocol, Response, ResponseBody, Role, Status, Stream,
    Version,
};
use crate::names::MessageSegment;
use crate::versions::MinorVersionTable;

/// Resolved protocol generation plan.
#[derive(Debug, Clone, PartialEq)]
pub struct ProtocolPlan<'d> {
    pub protocol: &'d Protocol,
    pub channel_id: u32,
    /// Latest minor in the major version.
    pub version: Version,
    /// Required protocol core version.
    pub core: Version,
    pub versions: Vec<VersionPlan<'d>>,
    /// Message plans grouped by sender role: client first, then provider.
    pub messages: Vec<MessagePlan<'d>>,
    pub open: Option<Encoding<'d>>,
    pub ack: Option<Encoding<'d>>,
    pub refusal: Option<Encoding<'d>>,
}

/// Generation metadata for one protocol minor.
#[derive(Debug, Clone, PartialEq)]
pub struct VersionPlan<'d> {
    pub minor: u16,
    pub fingerprint: u32,
    pub status: &'d Status,
}

/// Resolved protocol message generation plan.
#[derive(Debug, Clone, PartialEq)]
pub struct MessagePlan<'d> {
    pub message: &'d Message,
    /// Message ID within its receiver.
    pub id: u32,
    /// Introducing minor version.
    pub since: u16,
    pub sender: &'d Peer,
    pub receiver: &'d Peer,
    pub body: FieldsPlan<'d>,
    /// Stream following the message body.
    pub stream: Option<StreamPlan<'d>>,
    /// Response plan, or `None` for notifications.
    pub response: Option<ResponsePlan<'d>>,
    /// Maximum encoded client message frame size.
    pub limit: u32,
}

/// Resolved message response generation plan.
#[derive(Debug, Clone, PartialEq)]
pub struct ResponsePlan<'d> {
    pub body: ResponseBodyPlan<'d>,
    pub failure: Option<Encoding<'d>>,
}

/// Resolved successful response body layout.
#[derive(Debug, Clone, PartialEq)]
pub enum ResponseBodyPlan<'d> {
    Value(Encoding<'d>),
    Stream(StreamPlan<'d>),
}

/// Resolved stream layout.
#[derive(Debug, Clone, PartialEq)]
pub struct StreamPlan<'d> {
    pub start: Option<Encoding<'d>>,
    pub end: Option<Encoding<'d>>,
    pub item: Encoding<'d>,
    pub items: Length,
    /// Maximum encoded item size visible to the receiver. The frame limit caps client streams; provider streams use
    /// the item type bound.
    pub item_size: Option<u32>,
}

impl<'d> StreamPlan<'d> {
    fn of(definition: &'d Definition, stream: &'d Stream, sender: Role, limit: u32) -> Self {
        let encoding = |ty: &'d Option<_>| ty.as_ref().map(|ty| Encoding::of(definition, ty));
        let item_size = definition
            .max_size(&stream.item)
            .map(|size| u32::try_from(size).unwrap_or(u32::MAX));

        Self {
            start: encoding(&stream.start),
            end: encoding(&stream.end),
            item: Encoding::of(definition, &stream.item),
            items: stream.items,
            item_size: match sender {
                Role::Client => Some(item_size.map_or(limit, |size| size.min(limit))),
                Role::Provider => item_size,
            },
        }
    }
}

impl<'d> ResponsePlan<'d> {
    fn of(definition: &'d Definition, response: &'d Response, sender: Role, limit: u32) -> Self {
        Self {
            body: match &response.body {
                ResponseBody::Value(ty) => ResponseBodyPlan::Value(Encoding::of(definition, ty)),
                ResponseBody::Stream(stream) => {
                    ResponseBodyPlan::Stream(StreamPlan::of(definition, stream, sender, limit))
                }
            },
            failure: response
                .failure
                .as_ref()
                .map(|failure| Encoding::of(definition, failure)),
        }
    }
}

impl<'d> ProtocolPlan<'d> {
    pub(crate) fn of(major: MinorVersionTable<'d>, protocol: &'d Protocol) -> Self {
        let latest = major.latest();
        let definition: &'d Definition = &latest.definition;
        let source = Source {
            definition,
            additions: &latest.additions,
        };
        let messages = protocol
            .messages()
            .iter()
            .map(|message| {
                let sender = protocol.peer(message.sender());
                let receiver = protocol.peer(message.receiver);
                let limit = protocol.limit_of(message);

                MessagePlan {
                    message,
                    id: message.id,
                    since: latest.additions.message(message),
                    sender,
                    receiver,
                    body: FieldsPlan::of(source, &message.fields),
                    limit,
                    stream: message
                        .stream
                        .as_ref()
                        .map(|field| StreamPlan::of(definition, &field.stream, sender.role, limit)),
                    response: message.response.as_ref().map(|response| {
                        ResponsePlan::of(definition, response, receiver.role, limit)
                    }),
                }
            })
            .collect();
        let Some(core) = definition.core() else {
            unreachable!("the constraints give every protocol a core");
        };
        let encoding = |payload: &'d Option<_>| {
            payload
                .as_ref()
                .map(|payload| Encoding::of(definition, payload))
        };

        Self {
            protocol,
            channel_id: protocol.channel_id(),
            version: latest.version,
            core,
            versions: major
                .all()
                .iter()
                .map(|version| VersionPlan {
                    minor: version.version.minor,
                    fingerprint: version.fingerprint,
                    status: version.definition.status(),
                })
                .collect(),
            messages,
            open: encoding(&protocol.handshake.open),
            ack: encoding(&protocol.handshake.ack),
            refusal: encoding(&protocol.handshake.refusal),
        }
    }

    #[must_use]
    pub fn role(&self, role: Role) -> RolePlan<'_, 'd> {
        RolePlan { plan: self, role }
    }

    #[must_use]
    pub fn has_responses(&self, responses: bool) -> bool {
        self.messages
            .iter()
            .any(|message| message.response.is_some() == responses)
    }

    pub fn inbound_messages(&self, role: Role) -> impl Iterator<Item = &MessagePlan<'d>> {
        self.messages
            .iter()
            .filter(move |message| message.message.receiver == role)
    }
}

impl<'d> MessagePlan<'d> {
    #[must_use]
    pub fn has_response(&self) -> bool {
        self.response.is_some()
    }

    #[must_use]
    pub fn response_value(&self) -> Option<&Encoding<'d>> {
        match &self.response.as_ref()?.body {
            ResponseBodyPlan::Value(value) => Some(value),
            ResponseBodyPlan::Stream(_) => None,
        }
    }

    #[must_use]
    pub fn response_stream(&self) -> Option<&StreamPlan<'d>> {
        match &self.response.as_ref()?.body {
            ResponseBodyPlan::Value(_) => None,
            ResponseBodyPlan::Stream(stream) => Some(stream),
        }
    }

    #[must_use]
    pub fn failure(&self) -> Option<&Encoding<'d>> {
        self.response.as_ref()?.failure.as_ref()
    }

    #[must_use]
    pub fn has_segment(&self, segment: MessageSegment) -> bool {
        match segment {
            MessageSegment::Message => true,
            MessageSegment::MessageItem | MessageSegment::MessageEnd => self.stream.is_some(),
            MessageSegment::ResponseEnd => self.response.is_some(),
            MessageSegment::ResponseStart | MessageSegment::ResponseItem => {
                self.response_stream().is_some()
            }
            MessageSegment::Failure => self.failure().is_some(),
        }
    }

    #[must_use]
    pub fn segment(&self, segment: MessageSegment) -> Option<&Encoding<'d>> {
        match segment {
            MessageSegment::Message => None,
            MessageSegment::MessageItem => self.stream.as_ref().map(|stream| &stream.item),
            MessageSegment::MessageEnd => self.stream.as_ref()?.end.as_ref(),
            MessageSegment::ResponseEnd => match &self.response.as_ref()?.body {
                ResponseBodyPlan::Value(value) => Some(value),
                ResponseBodyPlan::Stream(stream) => stream.end.as_ref(),
            },
            MessageSegment::ResponseStart => self.response_stream()?.start.as_ref(),
            MessageSegment::ResponseItem => self.response_stream().map(|stream| &stream.item),
            MessageSegment::Failure => self.failure(),
        }
    }
}

/// Protocol plan viewed for one role.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RolePlan<'p, 'd> {
    /// Complete protocol plan.
    pub plan: &'p ProtocolPlan<'d>,
    /// Local role.
    pub role: Role,
}

impl<'p, 'd> RolePlan<'p, 'd> {
    #[must_use]
    pub fn protocol(&self) -> &'d Protocol {
        self.plan.protocol
    }

    #[must_use]
    pub fn local(&self) -> &'d Peer {
        self.plan.protocol.peer(self.role)
    }

    #[must_use]
    pub fn remote(&self) -> &'d Peer {
        self.plan.protocol.peer(self.role.remote())
    }

    pub fn inbound_messages(&self) -> impl Iterator<Item = &'p MessagePlan<'d>> + use<'p, 'd> {
        self.plan.inbound_messages(self.role)
    }
}
