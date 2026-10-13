use super::{BaseType, TypeMeta};
use crate::error::TypeParseError;

pub const MAX_TYPE_DEPTH: usize = 64;

pub(super) fn parse(text: &str) -> Result<TypeMeta, TypeParseError> {
    let mut parser = Parser { text, position: 0 };
    let meta = parser.type_meta(0)?;

    parser.skip_whitespace();
    if parser.position < text.len() {
        return Err(TypeParseError::TrailingText {
            position: parser.position,
        });
    }

    Ok(meta)
}

pub(super) fn is_name(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(is_name_byte)
}

fn is_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')
}

struct Parser<'a> {
    text: &'a str,
    position: usize,
}

impl<'a> Parser<'a> {
    fn remainder(&self) -> &'a str {
        &self.text[self.position..]
    }

    fn skip_whitespace(&mut self) {
        let remainder = self.remainder();

        self.position += remainder.len() - remainder.trim_start().len();
    }

    fn consume(&mut self, character: char) -> bool {
        self.skip_whitespace();

        let found = self.remainder().starts_with(character);

        if found {
            self.position += character.len_utf8();
        }

        found
    }

    fn expect(&mut self, character: char) -> Result<(), TypeParseError> {
        if self.consume(character) {
            Ok(())
        } else {
            Err(TypeParseError::MissingChar {
                character,
                position: self.position,
            })
        }
    }

    fn name(&mut self) -> Result<&'a str, TypeParseError> {
        self.skip_whitespace();

        let remainder = self.remainder();
        let length = remainder
            .bytes()
            .take_while(|&byte| is_name_byte(byte))
            .count();

        if length == 0 {
            return Err(TypeParseError::ExpectedName {
                position: self.position,
            });
        }

        self.position += length;

        Ok(&remainder[..length])
    }

    fn type_meta(&mut self, depth: usize) -> Result<TypeMeta, TypeParseError> {
        let name = self.name()?;
        let base = BaseType::from_name(name);
        let takes_type_arguments =
            matches!(base, Some(BaseType::List | BaseType::Map)) && self.consume('<');

        if !takes_type_arguments {
            return Ok(TypeMeta::from_name(name));
        }

        if depth == MAX_TYPE_DEPTH {
            return Err(TypeParseError::DepthLimit {
                position: self.position,
            });
        }

        let first = Box::new(self.type_meta(depth + 1)?);
        let meta = if base == Some(BaseType::Map) {
            self.expect(',')?;

            TypeMeta::Map {
                key: first,
                value: Box::new(self.type_meta(depth + 1)?),
            }
        } else {
            TypeMeta::List(first)
        };

        self.expect('>')?;

        Ok(meta)
    }
}

#[cfg(test)]
#[path = "tests/parser.rs"]
mod tests;
