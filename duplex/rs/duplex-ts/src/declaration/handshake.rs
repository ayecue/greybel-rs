use duplex_protocol::{Encoding, ProtocolPlan, Symbol};
use oxc_ast::ast::{ClassElement, Statement, TSType};
use oxc_span::SPAN;

use super::members::{MemberBuilder, READER_TYPE, WRITER_TYPE};

const UNUSED_WRITER: &str = "_writer";
const UNUSED_READER: &str = "_reader";

use crate::ast::{ModifierContainer, Parameter, Ts};
use crate::codec::Codec;
use crate::types::TsTypeMapper;

pub(crate) struct HandshakeClass<'p, 't, 'a> {
    ts: &'t Ts<'a>,
    codec: &'t Codec<'t, 'a>,
    plan: &'p ProtocolPlan<'p>,
}

impl<'p, 't, 'a> HandshakeClass<'p, 't, 'a> {
    pub(crate) fn new(
        ts: &'t Ts<'a>,
        codec: &'t Codec<'t, 'a>,
        plan: &'p ProtocolPlan<'p>,
    ) -> Self {
        Self { ts, codec, plan }
    }

    pub(crate) fn payload(ts: &Ts<'a>, payload: Option<&Encoding<'_>>) -> TSType<'a> {
        payload.map_or_else(
            || ts.undefined_type(),
            |payload| TsTypeMapper::new(ts).of(payload),
        )
    }

    pub(crate) fn declaration(&self) -> Statement<'a> {
        let ts = self.ts;
        let protocol = self.plan.protocol;
        let mut elements = Vec::new();

        for (name, value, placeholder, payload) in [
            ("Open", "open", "_open", &self.plan.open),
            ("Ack", "ack", "_ack", &self.plan.ack),
            ("Refusal", "refusal", "_refusal", &self.plan.refusal),
        ] {
            elements.extend(self.methods(name, [value, placeholder], payload.as_ref()));
        }

        let docs = ts.doc(format!(
            "Serializes and deserializes `{}` handshake payloads.",
            protocol.name
        ));

        ts.class(
            docs,
            &Symbol::Handshake(protocol).pascal(),
            false,
            None,
            elements,
        )
    }

    fn methods(
        &self,
        name: &str,
        [value, placeholder]: [&'static str; 2],
        payload: Option<&Encoding<'_>>,
    ) -> [ClassElement<'a>; 2] {
        let ts = self.ts;
        let ty = || Self::payload(ts, payload);
        let (write, read, writer, reader, parameter) = match payload {
            Some(payload) => {
                self.codec.reset_locals();

                let write = self.codec.write_value(payload, value);

                self.codec.reset_locals();

                let mut read = self.codec.read_value(payload, value);

                read.push(ts.return_(Some(ts.id(value))));

                (
                    write,
                    read,
                    MemberBuilder::writer_parameter(ts),
                    MemberBuilder::reader_parameter(ts),
                    value,
                )
            }
            None => (
                Vec::new(),
                vec![ts.return_(Some(ts.undefined_literal()))],
                Parameter::new(UNUSED_WRITER, ts.ty(WRITER_TYPE, Vec::new())),
                Parameter::new(UNUSED_READER, ts.ty(READER_TYPE, Vec::new())),
                placeholder,
            ),
        };

        [
            ts.method(
                SPAN,
                &format!("write{name}"),
                vec![writer, Parameter::new(parameter, ty())],
                ts.void(),
                write,
                ModifierContainer::STATIC,
            ),
            ts.method(
                SPAN,
                &format!("read{name}"),
                vec![reader],
                ty(),
                read,
                ModifierContainer::STATIC,
            ),
        ]
    }
}
