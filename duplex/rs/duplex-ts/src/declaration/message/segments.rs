use duplex_protocol::{Encoding, MessagePlan, MessageSegment, ResponseBodyPlan};
use oxc_ast::ast::{ClassElement, TSType};
use oxc_span::SPAN;

use super::MessageClass;
use crate::ast::{ModifierContainer, Parameter, Ts};
use crate::declaration::members::MemberBuilder;
use crate::types::TsTypeMapper;

const VALUE: &str = "value";
const VALUE_RESPONSE: &str = "ValueResponse";
const STREAM_RESPONSE: &str = "StreamResponse";
const STREAM_EXCHANGE: &str = "StreamExchange";

impl<'p, 't, 'a> MessageClass<'p, 't, 'a> {
    pub(crate) fn segment_type(
        ts: &Ts<'a>,
        plan: &MessagePlan<'_>,
        segment: MessageSegment,
    ) -> TSType<'a> {
        plan.segment(segment)
            .map_or_else(|| ts.void(), |encoding| TsTypeMapper::new(ts).of(encoding))
    }

    pub(crate) fn exchange(ts: &Ts<'a>, plan: &MessagePlan<'_>) -> Option<TSType<'a>> {
        let response = plan.response.as_ref()?;
        let segment = |segment| Self::segment_type(ts, plan, segment);
        let mut arguments = match &response.body {
            ResponseBodyPlan::Value(_) => vec![segment(MessageSegment::ResponseEnd)],
            ResponseBodyPlan::Stream(_) => vec![
                segment(MessageSegment::ResponseStart),
                segment(MessageSegment::ResponseEnd),
                segment(MessageSegment::ResponseItem),
            ],
        };

        if plan.failure().is_some() {
            arguments.push(segment(MessageSegment::Failure));
        }

        let base = match &response.body {
            ResponseBodyPlan::Value(_) => VALUE_RESPONSE,
            ResponseBodyPlan::Stream(_) => STREAM_RESPONSE,
        };
        let exchange = ts.ty(base, arguments);

        if plan.stream.is_none() {
            return Some(exchange);
        }

        Some(ts.ty(
            STREAM_EXCHANGE,
            vec![
                segment(MessageSegment::MessageEnd),
                segment(MessageSegment::MessageItem),
                exchange,
            ],
        ))
    }

    pub(super) fn segment_methods(
        &self,
        segment: MessageSegment,
        encoding: &Encoding<'_>,
    ) -> [ClassElement<'a>; 2] {
        let ts = self.ts;
        let types = TsTypeMapper::new(ts);

        self.codec.reset_locals();

        let write = self.codec.write_value(encoding, VALUE);

        self.codec.reset_locals();

        let mut read = self.codec.read_value(encoding, VALUE);

        read.push(ts.return_(Some(ts.id(VALUE))));

        let name = segment.pascal();

        [
            ts.method(
                SPAN,
                &format!("write{name}"),
                vec![
                    MemberBuilder::writer_parameter(ts),
                    Parameter::new(VALUE, types.of(encoding)),
                ],
                ts.void(),
                write,
                ModifierContainer::STATIC,
            ),
            ts.method(
                SPAN,
                &format!("read{name}"),
                vec![MemberBuilder::reader_parameter(ts)],
                types.of(encoding),
                read,
                ModifierContainer::STATIC,
            ),
        ]
    }
}
