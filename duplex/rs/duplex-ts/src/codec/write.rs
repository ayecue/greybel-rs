use duplex_protocol::{BlockPlan, Encoding, FieldPlan, FieldValue, FieldsPlan, Length};
use oxc_ast::ast::{BinaryOperator, Expression, Statement};

use super::{ABSENT, Codec, PRESENT, Side};
use crate::types::TsTypeMapper;

const ITEMS: &str = "items";
const ITEM: &str = "item";
const RECORD: &str = "record";
const KEYS: &str = "keys";
const KEY: &str = "key";
const VALUE: &str = "value";
const LENGTH: &str = "length";
const OBJECT: &str = "Object";
const OBJECT_KEYS: &str = "keys";
const WRITE: &str = "write";

impl<'a> Codec<'_, 'a> {
    pub(crate) fn write_fields(&self, plan: &FieldsPlan<'_>, source: &str) -> Vec<Statement<'a>> {
        let mut statements = Vec::new();

        for block in &plan.blocks {
            let block_statements = self.write_block(block, source);

            match block.since {
                0 => statements.extend(block_statements),
                since => statements.push(self.ts.if_(
                    self.writer_is_at_least(since),
                    block_statements,
                    None,
                )),
            }
        }

        statements
    }

    fn write_block(&self, block: &BlockPlan<'_>, source: &str) -> Vec<Statement<'a>> {
        let ts = self.ts;
        let mut statements = Vec::new();

        for byte in 0..block.flags.len() {
            let flags = block
                .fields
                .iter()
                .flat_map(|field| self.flag_terms(field, source, byte))
                .reduce(|left, right| ts.binary(left, BinaryOperator::BitwiseOR, right))
                .unwrap_or_else(|| ts.number_literal(0));

            statements.push(ts.expression(self.writer("byte", vec![flags])));
        }

        for field in &block.fields {
            statements.extend(self.write_field(field, source));
        }

        statements
    }

    fn flag_terms(&self, field: &FieldPlan<'_>, source: &str, byte: usize) -> Vec<Expression<'a>> {
        let ts = self.ts;
        let value = Self::access(field, source);
        let optional = field.presence.present.is_some();
        let presence = |operator| ts.binary(ts.path(&value), operator, ts.undefined_literal());
        let non_null = || {
            if optional {
                ts.binary(
                    ts.path(&value),
                    BinaryOperator::Inequality,
                    ts.null_literal(),
                )
            } else {
                ts.binary(
                    ts.path(&value),
                    BinaryOperator::StrictInequality,
                    ts.null_literal(),
                )
            }
        };
        let flag = || {
            if field.presence.is_required() {
                ts.path(&value)
            } else {
                ts.binary(
                    ts.path(&value),
                    BinaryOperator::StrictEquality,
                    ts.boolean_literal(true),
                )
            }
        };
        let bits = [
            field
                .presence
                .present
                .map(|bit| (bit, presence(BinaryOperator::StrictInequality))),
            field.presence.non_null.map(|bit| (bit, non_null())),
            Self::flag(field).map(|bit| (bit, flag())),
        ];

        bits.into_iter()
            .flatten()
            .filter(|(bit, _)| bit.byte == byte)
            .map(|(bit, condition)| {
                ts.wrap_in_parens(ts.conditional(
                    condition,
                    ts.number_literal(bit.mask),
                    ts.number_literal(0),
                ))
            })
            .collect()
    }

    fn write_field(&self, field: &FieldPlan<'_>, source: &str) -> Vec<Statement<'a>> {
        let ts = self.ts;
        let ty = match &field.value {
            FieldValue::Flag(_) => return Vec::new(),
            FieldValue::Flatten { fields, .. } => {
                let field_statements = self.write_fields(fields, source);

                return self.within(Side::Writer, field.position, &field.ty, field_statements);
            }
            FieldValue::Value(ty) => ty,
        };
        let value = Self::access(field, source);
        let condition = match (field.presence.present, field.presence.non_null) {
            (None, None) => None,
            (Some(_), None) => Some((BinaryOperator::StrictInequality, ts.undefined_literal())),
            (None, Some(_)) => Some((BinaryOperator::StrictInequality, ts.null_literal())),
            (Some(_), Some(_)) => Some((BinaryOperator::Inequality, ts.null_literal())),
        };
        let write = self.write_value(ty, &value);
        let write = match condition {
            None => write,
            Some((operator, absent)) => {
                vec![ts.if_(ts.binary(ts.path(&value), operator, absent), write, None)]
            }
        };

        self.within(Side::Writer, field.position, &field.ty, write)
    }

    fn access(field: &FieldPlan<'_>, source: &str) -> String {
        format!("{source}.{}", TsTypeMapper::property(field.field))
    }

    fn write_length(&self, length: Expression<'a>, requirement: Length) -> Expression<'a> {
        if requirement.is_any() {
            return self.writer("length", vec![length]);
        }

        let mut arguments = vec![length];

        arguments.extend(self.length_arguments(requirement));

        self.writer("validatedLength", arguments)
    }

    pub(crate) fn write_value(&self, ty: &Encoding<'_>, value: &str) -> Vec<Statement<'a>> {
        let ts = self.ts;

        match ty {
            Encoding::Bool => vec![ts.expression(self.writer("boolean", vec![ts.path(value)]))],
            Encoding::Char => vec![ts.expression(self.writer("char", vec![ts.path(value)]))],
            Encoding::String(requirements) if !requirements.is_any() => {
                let mut arguments = vec![ts.path(value)];

                arguments.extend(self.string_arguments(requirements));

                vec![ts.expression(self.writer("validatedString", arguments))]
            }
            Encoding::String(_) => vec![ts.expression(self.writer("string", vec![ts.path(value)]))],
            Encoding::Bytes(length) if !length.is_any() => {
                let mut arguments = vec![ts.path(value)];

                arguments.extend(self.length_arguments(*length));

                vec![ts.expression(self.writer("validatedBytes", arguments))]
            }
            Encoding::Bytes(_) => vec![ts.expression(self.writer("bytes", vec![ts.path(value)]))],
            Encoding::Number(kind, _) => {
                let value = match self.bound_arguments(ty) {
                    Some(mut bounds) => {
                        bounds.insert(0, ts.path(value));

                        self.writer("validateRange", bounds)
                    }
                    None => ts.path(value),
                };

                vec![ts.expression(self.writer(kind.keyword(), vec![value]))]
            }
            Encoding::Enumeration(name) => vec![ts.expression(ts.call_method(
                ts.id(name),
                WRITE,
                vec![ts.id(super::WRITER), ts.path(value)],
            ))],
            Encoding::Struct(_) | Encoding::Union(_) => vec![ts.expression(ts.call_method(
                ts.path(value),
                WRITE,
                vec![ts.id(super::WRITER)],
            ))],
            Encoding::Nullable(inner) => {
                let local = self.local(VALUE);
                let mut present =
                    vec![ts.expression(self.writer("byte", vec![ts.number_literal(PRESENT)]))];

                present.extend(self.write_value(inner, &local));

                vec![
                    ts.const_(&local, None, ts.path(value)),
                    ts.if_(
                        ts.binary(
                            ts.id(&local),
                            BinaryOperator::StrictEquality,
                            ts.null_literal(),
                        ),
                        vec![ts.expression(self.writer("byte", vec![ts.number_literal(ABSENT)]))],
                        Some(present),
                    ),
                ]
            }
            Encoding::List(item, length) => {
                let items = self.local(ITEMS);
                let local = self.local(ITEM);
                let mut statements = vec![
                    ts.const_(&items, None, ts.path(value)),
                    ts.expression(self.write_length(ts.member(ts.id(&items), LENGTH), *length)),
                ];
                let body = self.write_value(item, &local);

                statements.extend(self.each_item(Side::Writer, item, body, |body| {
                    ts.for_of(&local, ts.id(&items), body)
                }));

                statements
            }
            Encoding::Record(item, length) => {
                let record = self.local(RECORD);
                let keys = self.local(KEYS);
                let key = self.local(KEY);
                let local = self.local(ITEM);
                let mut body = vec![
                    ts.expression(self.writer("string", vec![ts.id(&key)])),
                    ts.const_(
                        &local,
                        None,
                        ts.non_null(ts.index(ts.id(&record), ts.id(&key))),
                    ),
                ];

                body.extend(self.write_value(item, &local));

                let mut statements = vec![
                    ts.const_(&record, None, ts.path(value)),
                    ts.const_(
                        &keys,
                        None,
                        ts.call_method(ts.id(OBJECT), OBJECT_KEYS, vec![ts.id(&record)]),
                    ),
                    ts.expression(self.write_length(ts.member(ts.id(&keys), LENGTH), *length)),
                ];

                statements.extend(self.each_item(Side::Writer, item, body, |body| {
                    ts.for_of(&key, ts.id(&keys), body)
                }));

                statements
            }
        }
    }
}
