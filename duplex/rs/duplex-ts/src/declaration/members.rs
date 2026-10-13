use duplex_protocol::{Encoding, FieldsPlan};
use oxc_ast::ast::{BinaryOperator, ClassElement, Expression, Statement};
use oxc_span::SPAN;

use super::{BODY, BODY_KEY, FIELDS, INIT};
use crate::ast::{InterfaceProperty, ModifierContainer, Parameter, Ts};
use crate::codec::Codec;
use crate::types::TsTypeMapper;

pub(crate) const WRITER_TYPE: &str = "BinaryWriter";
pub(crate) const READER_TYPE: &str = "BinaryReader";

pub(crate) struct MemberBuilder<'p, 't, 'a> {
    ts: &'t Ts<'a>,
    codec: &'t Codec<'t, 'a>,
    fields: &'p FieldsPlan<'p>,
    class: String,
}

impl<'p, 't, 'a> MemberBuilder<'p, 't, 'a> {
    pub(crate) fn new(
        codec: &'t Codec<'t, 'a>,
        ts: &'t Ts<'a>,
        fields: &'p FieldsPlan<'p>,
        class: String,
    ) -> Self {
        Self {
            ts,
            codec,
            fields,
            class,
        }
    }

    pub(crate) fn has_properties(&self) -> bool {
        !self.fields.properties().is_empty()
    }

    pub(crate) fn fields_type(&self) -> String {
        format!("{}.{FIELDS}", self.class)
    }

    pub(crate) fn init_type(&self) -> String {
        let interface = if self.has_defaults() { INIT } else { FIELDS };

        format!("{}.{interface}", self.class)
    }

    fn has_defaults(&self) -> bool {
        self.fields
            .properties()
            .iter()
            .any(|field| field.default.is_some())
    }

    pub(crate) fn writer_parameter(ts: &Ts<'a>) -> Parameter<'a> {
        Parameter::new("writer", ts.ty(WRITER_TYPE, Vec::new()))
    }

    pub(crate) fn reader_parameter(ts: &Ts<'a>) -> Parameter<'a> {
        Parameter::new("reader", ts.ty(READER_TYPE, Vec::new()))
    }

    pub(crate) fn body(&self, variant: bool) -> Vec<ClassElement<'a>> {
        let ts = self.ts;

        if !self.has_properties() {
            return Vec::new();
        }

        let mut statements = Vec::new();

        if variant {
            statements.push(ts.expression(ts.call(ts.super_(), Vec::new())));
        }

        statements.push(
            ts.expression(ts.assign(ts.index(ts.this(), ts.id(BODY_KEY)), self.fill_defaults())),
        );

        vec![
            ts.computed_property(
                ts.id(BODY_KEY),
                ts.ty(&self.fields_type(), Vec::new()),
                ModifierContainer::PROTECTED_READONLY,
            ),
            ts.constructor(
                vec![Parameter::new(BODY, ts.ty(&self.init_type(), Vec::new()))],
                statements,
            ),
        ]
    }

    pub(crate) fn getters(&self) -> Vec<ClassElement<'a>> {
        let ts = self.ts;
        let types = TsTypeMapper::new(ts);

        self.fields
            .properties()
            .into_iter()
            .map(|field| {
                let property = TsTypeMapper::property(field.field);

                ts.getter(
                    ts.docs(&field.field.docs),
                    &property,
                    types.return_type(field),
                    vec![ts.return_(Some(
                        ts.member(ts.index(ts.this(), ts.id(BODY_KEY)), &property),
                    ))],
                )
            })
            .collect()
    }

    pub(crate) fn write(&self, is_override: bool, prefix: Vec<Statement<'a>>) -> ClassElement<'a> {
        let ts = self.ts;

        self.codec.reset_locals();

        let mut statements = prefix;

        if self.has_properties() {
            statements.push(ts.const_(BODY, None, ts.index(ts.this(), ts.id(BODY_KEY))));
            statements.extend(self.codec.write_fields(self.fields, BODY));
        }

        ts.method(
            SPAN,
            "write",
            vec![Self::writer_parameter(ts)],
            ts.void(),
            statements,
            ModifierContainer::NONE.with_override(is_override),
        )
    }

    pub(crate) fn read(&self) -> ClassElement<'a> {
        let ts = self.ts;

        self.codec.reset_locals();

        let (mut statements, fields) = self.codec.read_fields(self.fields);
        let arguments = if fields.is_empty() {
            Vec::new()
        } else {
            vec![
                ts.object(
                    fields
                        .iter()
                        .map(|field| (field.property.as_str(), ts.id(&field.local)))
                        .collect(),
                ),
            ]
        };

        statements.push(ts.return_(Some(ts.new_(ts.path(&self.class), arguments))));

        ts.method(
            SPAN,
            "read",
            vec![Self::reader_parameter(ts)],
            ts.ty(&self.class, Vec::new()),
            statements,
            ModifierContainer::STATIC,
        )
    }

    fn fill_defaults(&self) -> Expression<'a> {
        let ts = self.ts;

        if !self.has_defaults() {
            return ts.id(BODY);
        }

        let types = TsTypeMapper::new(ts);
        let default_props: Vec<_> = self
            .fields
            .properties()
            .into_iter()
            .filter_map(|field| {
                Some((
                    TsTypeMapper::property(field.field),
                    field,
                    field.default.as_ref()?,
                ))
            })
            .collect();
        let properties = default_props
            .iter()
            .map(|(property, field, default)| {
                let body_access = || ts.member(ts.id(BODY), property);
                let default = types.value(default, &field.ty);
                let value = if matches!(field.ty, Encoding::Nullable(_)) {
                    let left_out = ts.binary(
                        body_access(),
                        BinaryOperator::StrictEquality,
                        ts.undefined_literal(),
                    );

                    ts.conditional(left_out, default, body_access())
                } else {
                    ts.coalesce(body_access(), default)
                };

                (property.as_str(), value)
            })
            .collect();

        ts.spread(ts.id(BODY), properties)
    }

    pub(crate) fn interfaces(&self) -> Vec<Statement<'a>> {
        if !self.has_properties() {
            return Vec::new();
        }

        let mut interfaces = vec![self.interface(FIELDS, false)];

        if self.has_defaults() {
            interfaces.push(self.interface(INIT, true));
        }

        interfaces
    }

    fn interface(&self, name: &str, defaults_optional: bool) -> Statement<'a> {
        let ts = self.ts;
        let types = TsTypeMapper::new(ts);
        let properties = self
            .fields
            .properties()
            .into_iter()
            .map(|field| InterfaceProperty {
                name: TsTypeMapper::property(field.field),
                optional: field.field.optional || (defaults_optional && field.default.is_some()),
                ty: types.of(&field.ty),
            })
            .collect();

        ts.interface(name, properties)
    }
}
