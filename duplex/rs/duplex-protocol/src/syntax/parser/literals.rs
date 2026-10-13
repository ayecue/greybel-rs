use super::{FALSE, NULL, Parser, TRUE};
use crate::error::DefinitionError;
use crate::model::Literal;
use crate::syntax::lexer::{Token, TokenKind};

const SIZES: &[(&str, u64)] = &[
    ("B", 1),
    ("KiB", 1 << 10),
    ("MiB", 1 << 20),
    ("GiB", 1 << 30),
];

impl Parser {
    pub(super) fn literal(&mut self) -> Result<Literal, DefinitionError> {
        let literal = match &self.peek().kind {
            TokenKind::Ident(word) => Literal::from_word(word),
            TokenKind::Text(text) => Literal::Text(text.clone()),
            TokenKind::Number(_) | TokenKind::Punct('-') => return self.number_literal(),
            _ => return Err(self.token_error("a value")),
        };

        self.bump();

        Ok(literal)
    }

    pub(super) fn number_literal(&mut self) -> Result<Literal, DefinitionError> {
        let mut number = String::new();

        if self.consume('-') {
            number.push('-');
        }

        number.push_str(&self.digits()?);
        if self.is('.')
            && matches!(
                self.tokens.get(self.position + 1),
                Some(Token {
                    kind: TokenKind::Number(_),
                    ..
                })
            )
        {
            self.bump();
            number.push('.');
            number.push_str(&self.digits()?);
        }

        Ok(Literal::Number(number))
    }

    pub(super) fn digits(&mut self) -> Result<String, DefinitionError> {
        match &self.peek().kind {
            TokenKind::Number(digits) => {
                let digits = digits.clone();

                self.bump();

                Ok(digits)
            }
            _ => Err(self.token_error("digits")),
        }
    }

    pub(super) fn duration(&mut self) -> Result<u32, DefinitionError> {
        self.with_unit(&[("ms", 1), ("s", 1_000), ("m", 60_000)], "a duration", 1)
    }

    pub(super) fn size(&mut self) -> Result<u32, DefinitionError> {
        self.with_unit(SIZES, "a size", 1)
    }

    pub(super) fn size_bound(&mut self) -> Result<u32, DefinitionError> {
        self.with_unit(SIZES, "a size", 0)
    }

    pub(super) fn count_bound(&mut self) -> Result<u32, DefinitionError> {
        self.with_unit(&[], "a count", 0)
    }

    fn with_unit(
        &mut self,
        units: &[(&str, u64)],
        what: &str,
        least: u32,
    ) -> Result<u32, DefinitionError> {
        let TokenKind::Number(token) = &self.peek().kind else {
            return Err(self.token_error(what));
        };
        let span = self.peek().span;
        let split = token
            .find(|char: char| !char.is_ascii_digit())
            .unwrap_or(token.len());
        let (digits, unit) = token.split_at(split);
        let scale = if unit.is_empty() {
            1
        } else if units.is_empty() {
            return Err(DefinitionError::new(
                span,
                format!("{what} has no unit: `{token}`"),
            ));
        } else {
            let names: Vec<&str> = units.iter().map(|(name, _)| *name).collect();

            units
                .iter()
                .find(|(name, _)| *name == unit)
                .map(|(_, scale)| *scale)
                .ok_or_else(|| {
                    DefinitionError::new(
                        span,
                        format!("`{unit}` is no unit of {what}: `{}`", names.join("`, `")),
                    )
                })?
        };
        let value = digits
            .parse::<u64>()
            .ok()
            .and_then(|digits| digits.checked_mul(scale))
            .and_then(|value| u32::try_from(value).ok())
            .filter(|&value| value >= least)
            .ok_or_else(|| {
                let range = if least == 0 { "" } else { "above 0 and " };

                DefinitionError::new(span, format!("`{token}` is {what} {range}below 2^32"))
            })?;

        self.bump();

        Ok(value)
    }

    pub(super) fn number(&mut self) -> Result<u32, DefinitionError> {
        match &self.peek().kind {
            TokenKind::Number(digits) => {
                let span = self.peek().span;
                let value = digits.parse().map_err(|_| {
                    DefinitionError::new(span, format!("`{digits}` is a number below 2^32"))
                })?;

                self.bump();

                Ok(value)
            }
            _ => Err(self.token_error("a number")),
        }
    }
}

impl Literal {
    fn from_word(word: &str) -> Self {
        match word {
            TRUE => Self::Bool(true),
            FALSE => Self::Bool(false),
            NULL => Self::Null,
            name => Self::Name(name.to_owned()),
        }
    }
}
