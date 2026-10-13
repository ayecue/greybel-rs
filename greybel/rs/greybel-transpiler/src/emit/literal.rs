use std::fmt;

use greybel_core::LiteralValue;
use greybel_core::syntax::Literal;
use greybel_symbols::delimiters::QUOTE;
use greybel_symbols::literals::NULL;

use crate::{BuildMode, TranspileOptions};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiteralStyle {
    Source,
    Short,
    Value,
    Output,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineBreakStyle<'a> {
    Retain,
    Join { function: &'a str, compact: bool },
}

#[derive(Debug, Clone, Copy)]
pub struct LiteralText<'a> {
    literal: &'a Literal,
    source: &'a str,
    style: LiteralStyle,
    line_breaks: LineBreakStyle<'a>,
}

impl<'a> LiteralText<'a> {
    #[must_use]
    pub fn new(literal: &'a Literal, source: &'a str, style: LiteralStyle) -> Self {
        Self {
            literal,
            source,
            style,
            line_breaks: LineBreakStyle::Retain,
        }
    }

    #[must_use]
    pub fn with_line_breaks(self, line_breaks: LineBreakStyle<'a>) -> Self {
        Self {
            line_breaks,
            ..self
        }
    }

    #[must_use]
    pub fn raw_text(&self) -> Option<&'a str> {
        match (&self.literal.value, self.style, self.literal.negated) {
            (LiteralValue::Nil, ..) => Some(NULL),
            (LiteralValue::String(_), ..) if self.joins() => None,
            (LiteralValue::String(_), ..) => Some(self.source),
            (LiteralValue::Number(_), LiteralStyle::Source | LiteralStyle::Short, false) => {
                Some(self.source)
            }
            _ => None,
        }
    }
}

impl fmt::Display for LiteralText<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.literal.negated { "-" } else { "" };

        match (&self.literal.value, self.style) {
            (LiteralValue::Nil, _) => f.write_str(NULL),
            (LiteralValue::Boolean(value), LiteralStyle::Short | LiteralStyle::Output) => {
                write!(f, "{sign}{}", u8::from(*value))
            }
            (LiteralValue::Boolean(value), _) => write!(f, "{sign}{value}"),
            (LiteralValue::Number(value), LiteralStyle::Value | LiteralStyle::Output) => {
                write!(f, "{sign}{value}")
            }
            (LiteralValue::Number(_), _) => write!(f, "{sign}{}", self.source),
            (LiteralValue::String(_), _) => match self.line_breaks {
                LineBreakStyle::Join { function, compact } if self.joins() => {
                    write_join(f, self.source, function, compact)
                }
                _ => f.write_str(self.source),
            },
        }
    }
}

impl<'a> LineBreakStyle<'a> {
    #[must_use]
    pub fn of(options: &TranspileOptions, function: &'a str) -> Self {
        if options.retain_multiline_strings {
            Self::Retain
        } else {
            Self::Join {
                function,
                compact: !matches!(options.mode, BuildMode::Beautify(_)),
            }
        }
    }

    #[must_use]
    pub fn joins(self) -> bool {
        matches!(self, Self::Join { .. })
    }
}

impl LiteralText<'_> {
    #[must_use]
    pub fn joins(&self) -> bool {
        self.line_breaks.joins()
            && matches!(self.literal.value, LiteralValue::String(_))
            && is_multiline(self.source)
    }
}

#[must_use]
pub fn is_multiline(source: &str) -> bool {
    source.contains(['\n', '\r'])
}

fn write_join(
    f: &mut fmt::Formatter<'_>,
    source: &str,
    function: &str,
    compact: bool,
) -> fmt::Result {
    let inner = &source[QUOTE.len()..source.len() - QUOTE.len()];
    let separator = if compact { "," } else { ", " };

    write!(f, "{function}([")?;

    for (index, line) in inner
        .split("\r\n")
        .flat_map(|line| line.split(['\n', '\r']))
        .enumerate()
    {
        if index > 0 {
            f.write_str(separator)?;
        }

        write!(f, "{QUOTE}{line}{QUOTE}")?;
    }

    f.write_str("])")
}
