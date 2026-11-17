use std::fmt;

use greybel_core::LiteralValue;
use greybel_core::lexer::NumericLiteral;
use greybel_core::syntax::Literal;
use greybel_symbols::delimiters::QUOTE;
use greybel_symbols::literals::NULL;

use crate::{BuildMode, MultilineStringStyle, TranspileOptions};

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
    /// `function(["a","b"])`.
    Join {
        function: &'a str,
        compact: bool,
    },
    /// `"a"+variable+"b"`, omitting empty lines.
    Concatenate {
        variable: &'a str,
        compact: bool,
    },
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
            (LiteralValue::String(_), ..) if self.rewrites() || !self.source.starts_with(QUOTE) => {
                None
            }
            (LiteralValue::String(_), ..) => Some(self.source),
            (LiteralValue::Number(_), LiteralStyle::Source | LiteralStyle::Short, false)
                if NumericLiteral::is_plain(self.source) =>
            {
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
            (LiteralValue::Number(value), _) if !NumericLiteral::is_plain(self.source) => {
                write!(f, "{sign}{value}")
            }
            (LiteralValue::Number(_), _) => write!(f, "{sign}{}", self.source),
            (LiteralValue::String(_), _) => match self.line_breaks {
                LineBreakStyle::Join { function, compact } if self.rewrites() => {
                    self.write_join(f, function, compact)
                }
                LineBreakStyle::Concatenate { variable, compact } if self.rewrites() => {
                    self.write_concatenation(f, variable, compact)
                }
                _ if !self.source.starts_with(QUOTE) => {
                    write!(f, "{QUOTE}{}{QUOTE}", self.source.replace(QUOTE, "\"\""))
                }
                _ => f.write_str(self.source),
            },
        }
    }
}

impl<'a> LineBreakStyle<'a> {
    /// `helper` is the name of the function or variable `options.multiline_strings` writes.
    #[must_use]
    pub fn of(options: &TranspileOptions, helper: &'a str) -> Self {
        let compact = !matches!(options.mode, BuildMode::Beautify(_));

        if options.retain_multiline_strings {
            return Self::Retain;
        }

        match options.multiline_strings {
            MultilineStringStyle::Join => Self::Join {
                function: helper,
                compact,
            },
            MultilineStringStyle::Concatenation => Self::Concatenate {
                variable: helper,
                compact,
            },
        }
    }

    /// Whether strings with line breaks are rewritten.
    #[must_use]
    pub fn rewrites(self) -> bool {
        !matches!(self, Self::Retain)
    }

    #[must_use]
    pub fn concatenates(self) -> bool {
        matches!(self, Self::Concatenate { .. })
    }
}

impl LiteralText<'_> {
    #[must_use]
    pub fn rewrites(&self) -> bool {
        self.line_breaks.rewrites()
            && matches!(self.literal.value, LiteralValue::String(_))
            && Self::is_multiline(self.source)
    }

    #[must_use]
    pub fn is_multiline(source: &str) -> bool {
        source.contains(['\n', '\r'])
    }

    /// Whether the text is a concatenation, which needs parentheses as an operand.
    #[must_use]
    pub fn concatenates(&self) -> bool {
        self.line_breaks.concatenates() && self.rewrites()
    }

    fn lines(&self) -> impl Iterator<Item = &str> {
        let source = self.source;

        source[QUOTE.len()..source.len() - QUOTE.len()]
            .split("\r\n")
            .flat_map(|line| line.split(['\n', '\r']))
    }

    fn write_concatenation(
        &self,
        f: &mut fmt::Formatter<'_>,
        variable: &str,
        compact: bool,
    ) -> fmt::Result {
        let plus = if compact { "+" } else { " + " };
        let mut is_first = true;

        for (index, line) in self.lines().enumerate() {
            if index > 0 {
                if !is_first {
                    f.write_str(plus)?;
                }

                f.write_str(variable)?;
                is_first = false;
            }

            if !line.is_empty() {
                if !is_first {
                    f.write_str(plus)?;
                }

                write!(f, "{QUOTE}{line}{QUOTE}")?;
                is_first = false;
            }
        }

        Ok(())
    }

    fn write_join(&self, f: &mut fmt::Formatter<'_>, function: &str, compact: bool) -> fmt::Result {
        let separator = if compact { "," } else { ", " };

        write!(f, "{function}([")?;

        for (index, line) in self.lines().enumerate() {
            if index > 0 {
                f.write_str(separator)?;
            }

            write!(f, "{QUOTE}{line}{QUOTE}")?;
        }

        f.write_str("])")
    }
}
