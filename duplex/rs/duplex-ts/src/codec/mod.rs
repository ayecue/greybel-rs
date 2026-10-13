mod read;
mod validations;
mod write;

use std::cell::Cell;

use duplex_protocol::{Bit, FieldPlan, FieldValue, Plan};
use oxc_ast::ast::{BinaryOperator, Expression, Statement};
pub(crate) use validations::Side;

use crate::ast::Ts;
use crate::types::TsTypeMapper;

const WRITER: &str = "writer";
const READER: &str = "reader";
const MINOR: &str = "minor";
pub(crate) const CHANNEL_ERROR: &str = "ChannelError";
pub(crate) const UNAVAILABLE_METHOD: &str = "unavailable";
const PRESENT: u8 = 1;
const ABSENT: u8 = 0;

pub(crate) struct Codec<'t, 'a> {
    ts: &'t Ts<'a>,
    plan: &'t Plan<'t>,
    types: TsTypeMapper<'t, 'a>,
    locals: Cell<usize>,
}

impl<'t, 'a> Codec<'t, 'a> {
    pub(crate) fn new(ts: &'t Ts<'a>, plan: &'t Plan<'t>) -> Self {
        Self {
            ts,
            plan,
            types: TsTypeMapper::new(ts),
            locals: Cell::new(0),
        }
    }

    pub(crate) fn plan(&self) -> &'t Plan<'t> {
        self.plan
    }

    pub(crate) fn reset_locals(&self) {
        self.locals.set(0);
    }

    fn local(&self, base: &str) -> String {
        let index = self.locals.get();

        self.locals.set(index + 1);

        format!("{base}{index}")
    }

    fn writer(&self, method: &str, arguments: Vec<Expression<'a>>) -> Expression<'a> {
        self.ts.call_method(self.ts.id(WRITER), method, arguments)
    }

    fn reader(&self, method: &str, arguments: Vec<Expression<'a>>) -> Expression<'a> {
        self.ts.call_method(self.ts.id(READER), method, arguments)
    }

    fn is_set(&self, flags: &[String], bit: Bit) -> Expression<'a> {
        let ts = self.ts;
        let bit_value = ts.binary(
            ts.id(&flags[bit.byte]),
            BinaryOperator::BitwiseAnd,
            ts.number_literal(bit.mask),
        );

        ts.binary(
            ts.wrap_in_parens(bit_value),
            BinaryOperator::StrictInequality,
            ts.number_literal(0),
        )
    }

    fn writer_is_at_least(&self, since: u16) -> Expression<'a> {
        let ts = self.ts;

        ts.binary(
            ts.member(ts.id(WRITER), MINOR),
            BinaryOperator::GreaterEqualThan,
            ts.number_literal(since),
        )
    }

    fn reader_is_at_least(&self, since: u16) -> Expression<'a> {
        let ts = self.ts;

        ts.binary(
            ts.member(ts.id(READER), MINOR),
            BinaryOperator::GreaterEqualThan,
            ts.number_literal(since),
        )
    }

    pub(crate) fn version_guard(&self, since: u16) -> Option<Statement<'a>> {
        let ts = self.ts;

        (since > 0).then(|| {
            ts.if_(
                ts.binary(
                    ts.member(ts.id(WRITER), MINOR),
                    BinaryOperator::LessThan,
                    ts.number_literal(since),
                ),
                vec![ts.throw(ts.call_method(
                    ts.id(CHANNEL_ERROR),
                    UNAVAILABLE_METHOD,
                    Vec::new(),
                ))],
                None,
            )
        })
    }

    fn flag(field: &FieldPlan<'_>) -> Option<Bit> {
        match field.value {
            FieldValue::Flag(bit) => Some(bit),
            _ => None,
        }
    }
}
