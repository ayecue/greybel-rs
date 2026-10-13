use duplex_protocol::{Encoding, Field, FieldPlan, Symbol, Value};
use oxc_ast::ast::{Expression, TSType};

use crate::ast::Ts;

const RECORD: &str = "Record";
const BYTES: &str = "Uint8Array";
const LOCAL_PREFIX: &str = "field";

pub(crate) struct TsTypeMapper<'t, 'a> {
    ts: &'t Ts<'a>,
    namespace: Option<&'t str>,
}

impl<'t, 'a> TsTypeMapper<'t, 'a> {
    pub(crate) fn new(ts: &'t Ts<'a>) -> Self {
        Self {
            ts,
            namespace: None,
        }
    }

    pub(crate) fn in_namespace(ts: &'t Ts<'a>, namespace: &'t str) -> Self {
        Self {
            ts,
            namespace: Some(namespace),
        }
    }

    pub(crate) fn declared_name(&self, name: &str) -> String {
        match self.namespace {
            Some(namespace) => format!("{namespace}.{name}"),
            None => name.to_owned(),
        }
    }

    pub(crate) fn of(&self, ty: &Encoding<'_>) -> TSType<'a> {
        let ts = self.ts;

        match ty {
            Encoding::Bool => ts.boolean(),
            Encoding::Char | Encoding::String(_) => ts.string(),
            Encoding::Number(kind, _) if kind.is_64_bit_integer() => ts.bigint(),
            Encoding::Number(..) => ts.number(),
            Encoding::Bytes(_) => ts.ty(BYTES, Vec::new()),
            Encoding::List(item, _) => ts.array(self.of(item)),
            Encoding::Record(value, _) => ts.ty(RECORD, vec![ts.string(), self.of(value)]),
            Encoding::Nullable(inner) => ts.union(vec![self.of(inner), ts.null()]),
            Encoding::Struct(name) | Encoding::Union(name) | Encoding::Enumeration(name) => {
                ts.ty(&self.declared_name(name), Vec::new())
            }
        }
    }

    pub(crate) fn value(&self, value: &Value, ty: &Encoding<'_>) -> Expression<'a> {
        let ts = self.ts;

        match (value, ty) {
            (Value::Null, _) => ts.null_literal(),
            (_, Encoding::Nullable(inner)) => self.value(value, inner),
            (Value::Bool(value), _) => ts.boolean_literal(*value),
            (Value::Text(text), _) => ts.string_literal(text),
            (Value::Char(char), _) => ts.string_literal(&char.to_string()),
            (Value::Integer(value), Encoding::Number(kind, _)) if kind.is_64_bit_integer() => {
                ts.bigint_literal(*value)
            }
            #[expect(
                clippy::cast_precision_loss,
                reason = "64-bit integers are bigints; other integers are exact JS numbers"
            )]
            (Value::Integer(value), _) => ts.number_literal(*value as f64),
            (Value::Float(value), _) => ts.number_literal(*value),
            (Value::Member { name, .. }, Encoding::Enumeration(enumeration)) => {
                ts.path(&format!("{}.{name}", self.declared_name(enumeration)))
            }
            (Value::Member { .. }, _) => unreachable!("the constraints bind values to their types"),
        }
    }

    pub(crate) fn return_type(&self, field: &FieldPlan<'_>) -> TSType<'a> {
        let ty = self.of(&field.ty);

        if field.field.optional {
            self.ts.union(vec![ty, self.ts.undefined_type()])
        } else {
            ty
        }
    }

    pub(crate) fn property(field: &Field) -> String {
        Symbol::Field(field).name().camel()
    }

    pub(crate) fn local(field: &Field) -> String {
        format!("{LOCAL_PREFIX}{}", Symbol::Field(field).pascal())
    }
}
