use duplex_protocol::{
    Definition, Encoding, Length, PatternTable, StringRequirementContainer, Value,
};
use oxc_ast::ast::{Expression, Statement};

use super::Codec;

const ENTER: &str = "enter";
const ADVANCE: &str = "advance";
const EXIT: &str = "exit";
const BEFORE_FIRST: i32 = -1;
const PATTERN: &str = "PATTERN_";
const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Side {
    Reader,
    Writer,
}

impl Side {
    fn object(self) -> &'static str {
        match self {
            Self::Reader => super::READER,
            Self::Writer => super::WRITER,
        }
    }
}

impl<'a> Codec<'_, 'a> {
    pub(crate) fn requires(&self, encoding: &Encoding<'_>) -> bool {
        self.plan.requires(encoding)
    }

    pub(crate) fn within(
        &self,
        side: Side,
        position: usize,
        encoding: &Encoding<'_>,
        statements: Vec<Statement<'a>>,
    ) -> Vec<Statement<'a>> {
        if !self.requires(encoding) {
            return statements;
        }

        let ts = self.ts;
        let object = side.object();
        let mut within = vec![ts.expression(ts.call_method(
            ts.id(object),
            ENTER,
            vec![ts.number_literal(f64::from(
                u32::try_from(position).expect("a position fits 32 bits"),
            ))],
        ))];

        within.extend(statements);
        within.push(ts.expression(ts.call_method(ts.id(object), EXIT, Vec::new())));

        within
    }

    pub(crate) fn each_item(
        &self,
        side: Side,
        item: &Encoding<'_>,
        body: Vec<Statement<'a>>,
        repeat: impl FnOnce(Vec<Statement<'a>>) -> Statement<'a>,
    ) -> Vec<Statement<'a>> {
        if !self.requires(item) {
            return vec![repeat(body)];
        }

        let ts = self.ts;
        let object = side.object();
        let mut each = vec![ts.expression(ts.call_method(ts.id(object), ADVANCE, Vec::new()))];

        each.extend(body);

        vec![
            ts.expression(ts.call_method(
                ts.id(object),
                ENTER,
                vec![ts.number_literal(BEFORE_FIRST)],
            )),
            repeat(each),
            ts.expression(ts.call_method(ts.id(object), EXIT, Vec::new())),
        ]
    }

    pub(crate) fn length_arguments(&self, length: Length) -> Vec<Expression<'a>> {
        let ts = self.ts;

        vec![
            ts.number_literal(length.min),
            length
                .max
                .map_or_else(|| ts.id("Infinity"), |max| ts.number_literal(max)),
        ]
    }

    pub(crate) fn string_arguments(
        &self,
        requirements: &StringRequirementContainer,
    ) -> Vec<Expression<'a>> {
        let ts = self.ts;
        let mut arguments = self.length_arguments(requirements.length);

        if let Some(pattern) = &requirements.pattern {
            arguments.push(ts.id(&self.pattern(pattern)));
        }

        arguments
    }

    pub(crate) fn bound_arguments(&self, encoding: &Encoding<'_>) -> Option<Vec<Expression<'a>>> {
        let ts = self.ts;
        let Encoding::Number(kind, bounds) = encoding else {
            return None;
        };

        if bounds.is_any() {
            return None;
        }

        let (min, max) =
            Definition::bounds_of(*kind, bounds).expect("the constraints verify bounds");
        #[expect(
            clippy::cast_precision_loss,
            reason = "64-bit integers are bigints; other integers are exact JS numbers"
        )]
        let bound = |value: Option<Value>, infinite: f64| match value {
            Some(Value::Integer(value)) if kind.is_64_bit_integer() => ts.bigint_literal(value),
            Some(Value::Integer(value)) => ts.number_literal(value as f64),
            Some(Value::Float(value)) => ts.number_literal(value),
            _ => ts.number_literal(infinite),
        };

        Some(vec![
            bound(min, f64::NEG_INFINITY),
            bound(max, f64::INFINITY),
        ])
    }

    pub(crate) fn pattern(&self, source: &str) -> String {
        let index = self
            .plan
            .patterns()
            .iter()
            .position(|pattern| *pattern == source)
            .expect("the plan lists every pattern");

        format!("{PATTERN}{index}")
    }

    pub(crate) fn patterns(&self) -> Vec<Statement<'a>> {
        let ts = self.ts;

        self.plan
            .patterns()
            .into_iter()
            .enumerate()
            .map(|(index, source)| {
                let table =
                    PatternTable::compile(source).expect("the constraints compile patterns");

                ts.const_(
                    &format!("{PATTERN}{index}"),
                    None,
                    ts.call_method(
                        ts.id("Pattern"),
                        "decode",
                        vec![ts.string_literal(&Self::base64(&Self::table_bytes(&table)))],
                    ),
                )
            })
            .collect()
    }

    fn table_bytes(table: &PatternTable) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&table.start.to_le_bytes());
        for length in [table.base.len(), table.next.len()] {
            let length = u32::try_from(length).expect("a pattern's table fits 32 bits");

            bytes.extend_from_slice(&length.to_le_bytes());
        }
        bytes.extend_from_slice(&table.classes);
        table
            .base
            .iter()
            .for_each(|base| bytes.extend_from_slice(&base.to_le_bytes()));
        table
            .next
            .iter()
            .for_each(|next| bytes.extend_from_slice(&next.to_le_bytes()));
        table
            .owner
            .iter()
            .for_each(|owner| bytes.extend_from_slice(&owner.to_le_bytes()));
        bytes.extend(table.accepting.iter().map(|&accepting| u8::from(accepting)));

        bytes
    }

    fn base64(bytes: &[u8]) -> String {
        let mut text = String::with_capacity(bytes.len().div_ceil(3) * 4);

        for chunk in bytes.chunks(3) {
            let word = chunk
                .iter()
                .enumerate()
                .fold(0_u32, |word, (index, &byte)| {
                    word | u32::from(byte) << (16 - 8 * index)
                });

            for index in 0..4 {
                if index <= chunk.len() {
                    text.push(BASE64[(word >> (18 - 6 * index) & 0x3f) as usize] as char);
                } else {
                    text.push('=');
                }
            }
        }

        text
    }
}
