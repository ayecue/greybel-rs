use duplex_protocol::{Bit, BlockPlan, Encoding, FieldPlan, FieldValue, FieldsPlan, Length};
use oxc_ast::ast::{Expression, Statement};

use super::{Codec, Side};
use crate::types::TsTypeMapper;

const FLAGS: &str = "flags";
const VALUE: &str = "value";
const ITEM: &str = "item";
const KEY: &str = "key";
const COUNT: &str = "count";
const PUSH: &str = "push";
const BINARY_READER: &str = "BinaryReader";
const SET_OBJECT_PROPERTY: &str = "setObjectProperty";
const READ: &str = "read";

pub(crate) struct ReadField {
    pub(crate) property: String,
    pub(crate) local: String,
}

impl<'a> Codec<'_, 'a> {
    pub(crate) fn read_fields(
        &self,
        plan: &FieldsPlan<'_>,
    ) -> (Vec<Statement<'a>>, Vec<ReadField>) {
        let ts = self.ts;
        let mut statements = Vec::new();
        let mut fields = Vec::new();

        for block in &plan.blocks {
            if block.since == 0 {
                let (read, block_fields) = self.read_block(block);

                statements.extend(read);
                fields.extend(block_fields);
                continue;
            }

            let (read, block_fields) = self.read_block_into(block);

            for plan in &block.fields {
                let local = TsTypeMapper::local(plan.field);
                let type_and_value = match &plan.default {
                    Some(default) => (self.types.of(&plan.ty), self.types.value(default, &plan.ty)),
                    None => (
                        ts.union(vec![self.types.of(&plan.ty), ts.undefined_type()]),
                        ts.undefined_literal(),
                    ),
                };

                statements.push(ts.let_(&local, type_and_value.0, type_and_value.1));
            }

            statements.push(ts.if_(self.reader_is_at_least(block.since), read, None));
            fields.extend(block_fields);
        }

        (statements, fields)
    }

    fn read_block(&self, block: &BlockPlan<'_>) -> (Vec<Statement<'a>>, Vec<ReadField>) {
        let mut statements = Vec::new();
        let flags = self.read_flags(block, &mut statements);
        let mut fields = Vec::new();

        for field in &block.fields {
            if let FieldValue::Flatten { fields: inner, .. } = &field.value {
                let (read, inner) = self.read_fields(inner);

                statements.extend(self.within(Side::Reader, field.position, &field.ty, read));
                fields.extend(inner);
                continue;
            }

            let local = TsTypeMapper::local(field.field);
            let read = self.read_field(field, &flags, &local);

            statements.extend(self.within(Side::Reader, field.position, &field.ty, read));
            fields.push(ReadField {
                property: TsTypeMapper::property(field.field),
                local,
            });
        }

        (statements, fields)
    }

    fn read_block_into(&self, block: &BlockPlan<'_>) -> (Vec<Statement<'a>>, Vec<ReadField>) {
        let ts = self.ts;
        let mut statements = Vec::new();
        let flags = self.read_flags(block, &mut statements);
        let mut fields = Vec::new();

        for field in &block.fields {
            let local = TsTypeMapper::local(field.field);
            let read = self.local(VALUE);
            let value = self.read_field(field, &flags, &read);

            statements.extend(self.within(Side::Reader, field.position, &field.ty, value));
            statements.push(ts.expression(ts.assign(ts.id(&local), ts.id(&read))));
            fields.push(ReadField {
                property: TsTypeMapper::property(field.field),
                local,
            });
        }

        (statements, fields)
    }

    fn read_flags(
        &self,
        block: &BlockPlan<'_>,
        statements: &mut Vec<Statement<'a>>,
    ) -> Vec<String> {
        let ts = self.ts;

        block
            .flags
            .iter()
            .map(|mask| {
                let local = self.local(FLAGS);

                statements.push(ts.const_(
                    &local,
                    None,
                    self.reader("flags", vec![ts.number_literal(*mask)]),
                ));

                local
            })
            .collect()
    }

    fn read_field(
        &self,
        field: &FieldPlan<'_>,
        flags: &[String],
        local: &str,
    ) -> Vec<Statement<'a>> {
        let ts = self.ts;
        let (value, ty) = match &field.value {
            FieldValue::Flag(bit) => (Some(self.is_set(flags, *bit)), None),
            FieldValue::Value(ty) => (self.read_expression(ty), Some(ty)),
            FieldValue::Flatten { .. } => unreachable!("flattened fields are read in their place"),
        };
        let Some(value) = value else {
            return ty.map_or_else(Vec::new, |ty| {
                self.read_field_statements(field, ty, flags, local)
            });
        };
        let value = match field.presence.non_null {
            Some(bit) => ts.conditional(self.is_set(flags, bit), value, ts.null_literal()),
            None => value,
        };
        let value = match field.presence.present {
            Some(bit) => ts.conditional(self.is_set(flags, bit), value, ts.undefined_literal()),
            None => value,
        };

        vec![ts.const_(local, None, value)]
    }

    fn read_field_statements(
        &self,
        field: &FieldPlan<'_>,
        ty: &Encoding<'_>,
        flags: &[String],
        local: &str,
    ) -> Vec<Statement<'a>> {
        let ts = self.ts;

        if field.presence.is_required() {
            return self.read_value(ty, local);
        }

        let mut type_list = vec![self.types.of(ty)];
        let mut initial = ts.undefined_literal();

        if field.presence.non_null.is_some() {
            type_list.push(ts.null());
            initial = ts.null_literal();
        }

        if field.presence.present.is_some() {
            type_list.push(ts.undefined_type());
            initial = ts.undefined_literal();
        }

        let read = |presence: Option<Bit>| {
            let value = self.local(VALUE);
            let mut statements = self.read_value(ty, &value);

            statements.push(ts.expression(ts.assign(ts.id(local), ts.id(&value))));

            match presence {
                Some(bit) => {
                    let alternate =
                        field.presence.present.is_some().then(|| {
                            vec![ts.expression(ts.assign(ts.id(local), ts.null_literal()))]
                        });

                    vec![ts.if_(self.is_set(flags, bit), statements, alternate)]
                }
                None => statements,
            }
        };
        let present = read(field.presence.non_null);
        let mut statements = vec![ts.let_(local, ts.union(type_list), initial)];

        match field.presence.present {
            Some(bit) => statements.push(ts.if_(self.is_set(flags, bit), present, None)),
            None => statements.extend(present),
        }

        statements
    }

    pub(crate) fn read_expression(&self, ty: &Encoding<'_>) -> Option<Expression<'a>> {
        let ts = self.ts;

        Some(match ty {
            Encoding::Bool => self.reader("boolean", Vec::new()),
            Encoding::Char => self.reader("char", Vec::new()),
            Encoding::String(requirements) if !requirements.is_any() => {
                self.reader("validatedString", self.string_arguments(requirements))
            }
            Encoding::String(_) => self.reader("string", Vec::new()),
            Encoding::Bytes(length) if !length.is_any() => {
                self.reader("validatedBytes", self.length_arguments(*length))
            }
            Encoding::Bytes(_) => self.reader("bytes", Vec::new()),
            Encoding::Number(kind, _) => {
                let value = self.reader(kind.keyword(), Vec::new());

                match self.bound_arguments(ty) {
                    Some(mut bounds) => {
                        bounds.insert(0, value);

                        self.reader("validateRange", bounds)
                    }
                    None => value,
                }
            }
            Encoding::Struct(name) | Encoding::Union(name) | Encoding::Enumeration(name) => {
                ts.call_method(ts.id(name), READ, vec![ts.id(super::READER)])
            }
            Encoding::Nullable(inner) => ts.conditional(
                self.reader("boolean", Vec::new()),
                self.read_expression(inner)?,
                ts.null_literal(),
            ),
            Encoding::List(..) | Encoding::Record(..) => return None,
        })
    }

    fn read_length(&self, length: Length) -> Expression<'a> {
        if length.is_any() {
            return self.reader("length", Vec::new());
        }

        self.reader("validatedLength", self.length_arguments(length))
    }

    pub(crate) fn read_value(&self, ty: &Encoding<'_>, target: &str) -> Vec<Statement<'a>> {
        let ts = self.ts;

        if let Some(value) = self.read_expression(ty) {
            return vec![ts.const_(target, None, value)];
        }

        match ty {
            Encoding::List(item, length) => {
                let count = self.local(COUNT);
                let body = match self.read_expression(item) {
                    Some(value) => {
                        vec![ts.expression(ts.call_method(ts.id(target), PUSH, vec![value]))]
                    }
                    None => {
                        let local = self.local(ITEM);
                        let mut body = self.read_value(item, &local);

                        body.push(ts.expression(ts.call_method(
                            ts.id(target),
                            PUSH,
                            vec![ts.id(&local)],
                        )));

                        body
                    }
                };
                let mut statements = vec![ts.const_(
                    target,
                    Some(self.types.of(ty)),
                    ts.array_literal(Vec::new()),
                )];
                let start = self.read_length(*length);

                statements.extend(self.each_item(Side::Reader, item, body, |body| {
                    ts.repeat(&count, start, body)
                }));

                statements
            }
            Encoding::Record(item, length) => {
                let count = self.local(COUNT);
                let key = self.local(KEY);
                let local = self.local(ITEM);
                let mut body = vec![ts.const_(&key, None, self.reader("string", Vec::new()))];

                body.extend(self.read_value(item, &local));
                body.push(ts.expression(ts.call_method(
                    ts.id(BINARY_READER),
                    SET_OBJECT_PROPERTY,
                    vec![ts.id(target), ts.id(&key), ts.id(&local)],
                )));

                let mut statements =
                    vec![ts.const_(target, Some(self.types.of(ty)), ts.object(Vec::new()))];
                let start = self.read_length(*length);

                statements.extend(self.each_item(Side::Reader, item, body, |body| {
                    ts.repeat(&count, start, body)
                }));

                statements
            }
            Encoding::Nullable(inner) => {
                let local = self.local(VALUE);
                let mut present = self.read_value(inner, &local);

                present.push(ts.expression(ts.assign(ts.id(target), ts.id(&local))));

                vec![
                    ts.let_(target, self.types.of(ty), ts.null_literal()),
                    ts.if_(self.reader("boolean", Vec::new()), present, None),
                ]
            }
            _ => unreachable!("the other values are read by an expression"),
        }
    }
}
