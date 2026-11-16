use greybel_symbols::literals::NULL;
use mono_number::NumberStyles;

use crate::ast::{AstNode, DumpContext, NodeFieldContainer, NodeId, TextSpan};
use crate::diagnostics::LexerDiagnostic;
use crate::error::ParseResult;
use crate::lexer::{NumericLiteral, TokenKind};
use crate::parser::Parser;
use crate::position::Span;
use crate::syntax::greybel::Template;

#[derive(Debug, Clone, PartialEq)]
pub enum LiteralValue {
    String(TextSpan),
    Number(f64),
    Boolean(bool),
    Nil,
}

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct Literal {
    pub value: LiteralValue,
    pub negated: bool,
}

impl AstNode for Literal {
    fn type_name(&self) -> &'static str {
        match self.value {
            LiteralValue::String(_) => "StringLiteral",
            LiteralValue::Number(_) => "NumericLiteral",
            LiteralValue::Boolean(_) => "BooleanLiteral",
            LiteralValue::Nil => "NilLiteral",
        }
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let sign = if self.negated { "-" } else { "" };
        let text = match &self.value {
            LiteralValue::String(text) => ctx.read(*text).into_owned(),
            LiteralValue::Number(n) => n.to_string(),
            LiteralValue::Boolean(b) => b.to_string(),
            LiteralValue::Nil => NULL.to_owned(),
        };

        format!("Literal[{}][{sign}{text}]", ctx.location())
    }
}

impl Literal {
    pub fn parse(parser: &mut Parser<'_>) -> ParseResult<NodeId> {
        let token = parser.token();
        let span = token.span();

        if token.as_template().is_some() {
            return Template::parse(parser);
        }

        let value = if let Some(string) = token.as_string() {
            LiteralValue::String(string.text())
        } else if let Some(&number) = token.as_number() {
            let source = parser.source();

            let extended = !NumericLiteral::is_plain(number.raw(source));

            LiteralValue::Number(if parser.should_parse_numbers_as_mono() && !extended {
                Self::parse_mono_number(parser, number, source, span)?
            } else {
                number.number(source)
            })
        } else if let Some(boolean) = token.as_boolean() {
            LiteralValue::Boolean(boolean.boolean())
        } else {
            debug_assert_eq!(
                token.kind(),
                TokenKind::NilLiteral,
                "Literal::parse on a non-literal token"
            );

            LiteralValue::Nil
        };
        let literal = parser.alloc(
            Literal {
                value,
                negated: false,
            },
            span,
        );

        parser
            .stream
            .chunk_data
            .current_mut()
            .literals
            .push(literal);
        parser.advance()?;

        Ok(literal)
    }

    fn parse_mono_number(
        parser: &mut Parser<'_>,
        number: NumericLiteral,
        source: &str,
        span: Span,
    ) -> ParseResult<f64> {
        const STYLES: NumberStyles = NumberStyles::NUMBER.union(NumberStyles::ALLOW_EXPONENT);
        let raw = number.raw(source);

        match mono_number::parse_f64(raw, STYLES) {
            Some(value) => Ok(value),
            None => {
                parser.report_at(LexerDiagnostic::InvalidNumericLiteral(raw.to_owned()), span)?;

                Ok(number.number(source))
            }
        }
    }
}
