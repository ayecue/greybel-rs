use duplex_protocol::{MessagePlan, MessageSegment, Role, RolePlan, Symbol};
use oxc_ast::ast::{Statement, TSType};
use oxc_span::SPAN;

use super::handshake::HandshakeClass;
use super::message::MessageClass;
use crate::ast::{ModifierContainer, Parameter, Ts, TypeParameter};

const ROUTER: &str = "Router";
const CHANNEL: &str = "Channel";
const NEXT: &str = "Next";
const INBOUND: &str = "Inbound";
const REPLY: &str = "Reply";
const ROUTED: &str = "Routed";
const ROUTED_MESSAGE: &str = "M";
const PROMISE: &str = "Promise";
const DRIVER: &str = "Driver";
const TRANSPORT: &str = "Transport";
const CHANNEL_OPTIONS: &str = "ChannelOptions";
const ROUTE_CONTEXT: &str = "RouteContext";
const INCOMING_STREAM: &str = "IncomingStream";
const RESPONSE_WRITER: &str = "ResponseWriter";

pub(crate) struct RoleDeclarationBuilder<'p, 't, 'a> {
    ts: &'t Ts<'a>,
    plan: RolePlan<'p, 'p>,
}

impl<'p, 't, 'a> RoleDeclarationBuilder<'p, 't, 'a> {
    pub(crate) fn new(ts: &'t Ts<'a>, plan: RolePlan<'p, 'p>) -> Self {
        Self { ts, plan }
    }

    pub(crate) fn declarations(&self) -> Vec<Statement<'a>> {
        vec![self.router(), self.channel()]
    }

    fn member(&self) -> String {
        format!(
            "{}.{}",
            Symbol::Peers(self.plan.protocol()).pascal(),
            self.plan.local().name
        )
    }

    fn peer_arguments(&self) -> Vec<TSType<'a>> {
        vec![
            self.ts.type_of(&self.plan.protocol().name),
            self.ts.ty(&self.member(), Vec::new()),
        ]
    }

    fn base(&self, base: &str) -> (oxc_ast::ast::Expression<'a>, Vec<oxc_ast::ast::TSType<'a>>) {
        (self.ts.id(base), self.peer_arguments())
    }

    fn routed_message(ts: &Ts<'a>) -> TSType<'a> {
        ts.ty(ROUTED_MESSAGE, Vec::new())
    }

    fn output(ts: &Ts<'a>, message: &MessagePlan<'_>) -> TSType<'a> {
        MessageClass::segment_type(ts, message, MessageSegment::ResponseEnd)
    }

    fn items_type(ts: &Ts<'a>, message: &MessagePlan<'_>) -> TSType<'a> {
        ts.ty(
            INCOMING_STREAM,
            vec![
                MessageClass::segment_type(ts, message, MessageSegment::MessageEnd),
                MessageClass::segment_type(ts, message, MessageSegment::MessageItem),
            ],
        )
    }

    fn stream_type(ts: &Ts<'a>, message: &MessagePlan<'_>) -> TSType<'a> {
        ts.ty(
            RESPONSE_WRITER,
            vec![
                MessageClass::segment_type(ts, message, MessageSegment::ResponseStart),
                MessageClass::segment_type(ts, message, MessageSegment::ResponseItem),
            ],
        )
    }

    fn router(&self) -> Statement<'a> {
        let ts = self.ts;
        let peer = self.plan.local();
        let next = |output| Parameter::new("next", ts.ty(NEXT, vec![output]));
        let mut elements: Vec<_> = self
            .plan
            .inbound_messages()
            .map(|message| {
                let output = || Self::output(ts, message);
                let mut parameters = vec![Parameter::new(
                    "message",
                    ts.ty(&message.message.name, Vec::new()),
                )];

                if message.stream.is_some() {
                    parameters.push(Parameter::new("_items", Self::items_type(ts, message)));
                }

                if message.response_stream().is_some() {
                    parameters.push(Parameter::new("_stream", Self::stream_type(ts, message)));
                }

                parameters.push(next(output()));
                if message.has_response() {
                    parameters.push(Parameter::new("_context", ts.ty(ROUTE_CONTEXT, Vec::new())));
                }

                ts.method(
                    SPAN,
                    &Symbol::Handler(message.message).name().camel(),
                    parameters,
                    ts.union(vec![output(), ts.ty(PROMISE, vec![output()])]),
                    vec![ts.return_(Some(ts.call(ts.id("next"), Vec::new())))],
                    ModifierContainer::NONE,
                )
            })
            .collect();
        let context = |property: &str| ts.member(ts.id("context"), property);
        let cases = self
            .plan
            .inbound_messages()
            .map(|message| {
                let name = &message.message.name;
                let mut arguments = vec![ts.as_(ts.id("message"), ts.ty(name, Vec::new()))];

                if message.stream.is_some() {
                    arguments
                        .push(ts.as_(context("incomingStream"), Self::items_type(ts, message)));
                }

                if message.response_stream().is_some() {
                    arguments
                        .push(ts.as_(context("responseWriter"), Self::stream_type(ts, message)));
                }

                arguments.push(ts.as_(ts.id("next"), ts.ty(NEXT, vec![Self::output(ts, message)])));
                if message.has_response() {
                    arguments.push(ts.id("context"));
                }

                let handle = ts.call_method(
                    ts.this(),
                    &Symbol::Handler(message.message).name().camel(),
                    arguments,
                );
                let routed = ts.ty(ROUTED, vec![Self::routed_message(ts)]);

                (
                    ts.member(ts.id(name), "id"),
                    vec![ts.return_(Some(ts.as_(handle, routed)))],
                )
            })
            .collect::<Vec<_>>();
        let forward = ts.return_(Some(ts.call(ts.id("next"), Vec::new())));
        let context_name = if self
            .plan
            .inbound_messages()
            .any(duplex_protocol::MessagePlan::has_response)
        {
            "context"
        } else {
            "_context"
        };
        let (type_id, message, body) = if cases.is_empty() {
            ("_typeId", "_message", vec![forward])
        } else {
            (
                "typeId",
                "message",
                vec![ts.switch(ts.id("typeId"), cases, vec![forward])],
            )
        };

        elements.extend(self.open_methods());
        elements.push(ts.generic_method(
            "route",
            vec![TypeParameter {
                name: ROUTED_MESSAGE,
                constraint: ts.ty(INBOUND, self.peer_arguments()),
            }],
            vec![
                Parameter::new(type_id, ts.number()),
                Parameter::new(message, Self::routed_message(ts)),
                next(ts.ty(REPLY, vec![Self::routed_message(ts)])),
                Parameter::new(context_name, ts.ty(ROUTE_CONTEXT, Vec::new())),
            ],
            ts.ty(ROUTED, vec![Self::routed_message(ts)]),
            body,
            ModifierContainer::OVERRIDE,
        ));

        let docs = ts.doc(format!(
            "Routes messages handled by the {} `{}`. Override a message method to handle it, or call `next()` to forward it.",
            self.plan.role.keyword(),
            peer.name
        ));

        ts.class(
            docs,
            &Symbol::Router(peer).pascal(),
            true,
            Some(self.base(ROUTER)),
            elements,
        )
    }

    fn open_methods(&self) -> Vec<oxc_ast::ast::ClassElement<'a>> {
        let ts = self.ts;
        let plan = self.plan.plan;

        if self.plan.role != Role::Provider || !plan.protocol.handshake.is_declared() {
            return Vec::new();
        }

        let open = || HandshakeClass::payload(ts, plan.open.as_ref());
        let ack = || HandshakeClass::payload(ts, plan.ack.as_ref());
        let on_connect = ts.abstract_method(
            "onConnect",
            vec![Parameter::new("open", open())],
            ts.union(vec![ack(), ts.ty(PROMISE, vec![ack()])]),
        );
        let route_open = ts.method(
            SPAN,
            "routeOpen",
            vec![
                Parameter::new("open", open()),
                Parameter::new("_next", ts.ty(NEXT, vec![ack()])),
            ],
            ts.union(vec![ack(), ts.ty(PROMISE, vec![ack()])]),
            vec![ts.return_(Some(ts.call_method(
                ts.this(),
                "onConnect",
                vec![ts.id("open")],
            )))],
            ModifierContainer::OVERRIDE,
        );

        vec![on_connect, route_open]
    }

    fn channel(&self) -> Statement<'a> {
        let ts = self.ts;
        let peer = self.plan.local();
        let protocol = &self.plan.protocol().name;
        let action = match self.plan.role {
            Role::Client => "connect to its provider",
            Role::Provider => "listen for its client",
        };
        let docs = ts.doc(format!(
            "The {} `{}` for `{protocol}` uses a transport or driver transport to {action}.",
            self.plan.role.keyword(),
            peer.name
        ));
        let opens = self.plan.role == Role::Client && self.plan.plan.open.is_some();
        let transport = ts.union(vec![
            ts.ty(TRANSPORT, Vec::new()),
            ts.ty(DRIVER, Vec::new()),
        ]);
        let mut parameters = vec![Parameter::new("transport", transport)];
        let mut arguments = vec![ts.id("transport"), ts.id(protocol), ts.path(&self.member())];

        if opens {
            parameters.push(Parameter::new(
                "open",
                HandshakeClass::payload(ts, self.plan.plan.open.as_ref()),
            ));
        }

        parameters.push(Parameter::optional(
            "options",
            ts.ty(CHANNEL_OPTIONS, Vec::new()),
        ));
        arguments.push(ts.id("options"));
        if opens {
            arguments.push(ts.id("open"));
        }

        let constructor = ts.constructor(
            parameters,
            vec![ts.expression(ts.call(ts.super_(), arguments))],
        );

        ts.class(
            docs,
            &Symbol::Channel(peer).pascal(),
            false,
            Some(self.base(CHANNEL)),
            vec![constructor],
        )
    }
}
