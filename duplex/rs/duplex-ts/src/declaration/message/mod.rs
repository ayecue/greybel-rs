mod segments;

use duplex_protocol::{MessagePlan, MessageSegment, Protocol, ProtocolPlan, Symbol};
use oxc_ast::ast::{ClassElement, Statement};
use oxc_span::SPAN;

use super::members::MemberBuilder;
use super::tags::TagExpressionContainer;
use crate::ast::{ModifierContainer, Parameter, Ts};
use crate::codec::Codec;

const MESSAGE: &str = "Message";
const OBJECT: &str = "object";

pub(crate) struct MessageClass<'p, 't, 'a> {
    ts: &'t Ts<'a>,
    codec: &'t Codec<'t, 'a>,
    protocol: &'p ProtocolPlan<'p>,
    plan: &'p MessagePlan<'p>,
}

impl<'p, 't, 'a> MessageClass<'p, 't, 'a> {
    pub(crate) fn new(
        ts: &'t Ts<'a>,
        codec: &'t Codec<'t, 'a>,
        protocol: &'p ProtocolPlan<'p>,
        plan: &'p MessagePlan<'p>,
    ) -> Self {
        Self {
            ts,
            codec,
            protocol,
            plan,
        }
    }

    fn sender(&self) -> String {
        format!(
            "{}.{}",
            Symbol::Peers(self.protocol.protocol).pascal(),
            self.plan.sender.name
        )
    }

    fn flags(&self) -> Vec<ClassElement<'a>> {
        let ts = self.ts;

        [
            ("responds", self.plan.has_response()),
            ("streams", self.plan.stream.is_some()),
            ("streamsResponse", self.plan.response_stream().is_some()),
            (
                "startsResponse",
                self.plan
                    .response_stream()
                    .is_some_and(|stream| stream.start.is_some()),
            ),
        ]
        .into_iter()
        .filter(|(_, set)| *set)
        .map(|(name, _)| {
            ts.property(
                SPAN,
                name,
                None,
                Some(ts.boolean_literal(true)),
                ModifierContainer::STATIC_READONLY.with_override(true),
            )
        })
        .collect()
    }

    pub(crate) fn declaration(&self) -> Vec<Statement<'a>> {
        let ts = self.ts;
        let message = self.plan.message;
        let name = &message.name;
        let members = MemberBuilder::new(self.codec, ts, &self.plan.body, name.clone());
        let fields = if members.has_properties() {
            ts.ty(&members.fields_type(), Vec::new())
        } else {
            ts.ty(OBJECT, Vec::new())
        };
        let sender = self.sender();
        let mut arguments = vec![ts.ty(&sender, Vec::new()), fields];

        arguments.extend(Self::exchange(ts, self.plan));

        let constructor = if members.has_properties() {
            ts.constructor(
                vec![Parameter::new(
                    "body",
                    ts.ty(&members.fields_type(), Vec::new()),
                )],
                vec![ts.expression(ts.call(ts.super_(), vec![ts.id("body")]))],
            )
        } else {
            ts.constructor(
                Vec::new(),
                vec![ts.expression(ts.call(ts.super_(), vec![ts.object(Vec::new())]))],
            )
        };
        let mut elements = vec![
            ts.property(
                SPAN,
                "id",
                None,
                Some(ts.number_literal(self.plan.id)),
                ModifierContainer::STATIC_READONLY,
            ),
            ts.property(
                SPAN,
                "since",
                None,
                Some(ts.number_literal(self.plan.since)),
                ModifierContainer::STATIC_READONLY,
            ),
            ts.property(
                SPAN,
                "sender",
                None,
                Some(ts.path(&sender)),
                ModifierContainer::STATIC_READONLY,
            ),
            constructor,
        ];

        if let Some(timeout) = message.timeout {
            elements.push(ts.property(
                SPAN,
                "timeout",
                None,
                Some(ts.number_literal(timeout)),
                ModifierContainer::STATIC_READONLY,
            ));
        }

        if self.plan.limit != Protocol::DEFAULT_LIMIT {
            elements.push(ts.property(
                SPAN,
                "limit",
                None,
                Some(ts.number_literal(self.plan.limit)),
                ModifierContainer::STATIC_READONLY.with_override(true),
            ));
        }

        let response_stream = self.plan.response_stream();

        for (name, stream) in [
            ("messageItemRange", self.plan.stream.as_ref()),
            ("responseItemRange", response_stream),
        ] {
            let Some(stream) = stream.filter(|stream| !stream.items.is_any()) else {
                continue;
            };
            let max = stream
                .items
                .max
                .map_or_else(|| ts.id("Infinity"), |max| ts.number_literal(max));

            elements.push(ts.property(
                SPAN,
                name,
                None,
                Some(ts.as_const(ts.array_literal(vec![ts.number_literal(stream.items.min), max]))),
                ModifierContainer::STATIC_READONLY.with_override(true),
            ));
        }

        for (name, stream) in [
            ("messageItemSize", self.plan.stream.as_ref()),
            ("responseItemSize", response_stream),
        ] {
            let Some(size) = stream.and_then(|stream| stream.item_size) else {
                continue;
            };

            elements.push(ts.property(
                SPAN,
                name,
                None,
                Some(ts.number_literal(size)),
                ModifierContainer::STATIC_READONLY.with_override(true),
            ));
        }

        let tags = TagExpressionContainer::new(ts, self.codec.plan().definition);

        elements.extend(tags.statics(&message.tags, tags.fields(&self.plan.body), true));
        elements.extend(members.getters());
        elements.push(members.write(true, Vec::new()));
        elements.push(members.read());
        elements.extend(self.flags());
        for segment in MessageSegment::ALL {
            if let Some(encoding) = self.plan.segment(segment) {
                elements.extend(self.segment_methods(segment, encoding));
            }
        }

        let mut statements = vec![ts.class(
            ts.docs(&message.docs),
            name,
            false,
            Some((ts.id(MESSAGE), arguments)),
            elements,
        )];
        let interfaces = members.interfaces();

        if !interfaces.is_empty() {
            statements.push(ts.namespace(name, interfaces));
        }

        statements
    }
}
