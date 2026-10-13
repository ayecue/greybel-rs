//! Position in a .NET pattern or replacement, including scan operations shared by `RegexParser` passes.

use std::sync::LazyLock;

use fancy_regex::Regex;

use super::error::Reason;

pub const WORD_MEMBERS: &str = r"\p{L}\p{Mn}\p{Nd}\p{Pc}";

const MAX_NUMBER: u32 = i32::MAX.unsigned_abs();

pub fn is_word_char(c: char) -> bool {
    static WORD_CHAR: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(&format!(r"\A[{WORD_MEMBERS}\x{{200C}}\x{{200D}}]\z")).expect("valid pattern")
    });

    if c.is_ascii() {
        return c.is_ascii_alphanumeric() || c == '_';
    }

    WORD_CHAR
        .is_match(c.encode_utf8(&mut [0; 4]))
        .unwrap_or(false)
}

fn is_whitespace(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\x0c' | '\r' | ' ')
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reference<'t> {
    Number(u32),
    Name(&'t str),
    Plain(u32),
}

#[derive(Debug, Clone)]
pub struct Cursor<'t> {
    text: &'t str,
    position: usize,
}

impl<'t> Cursor<'t> {
    pub fn new(text: &'t str) -> Self {
        Self { text, position: 0 }
    }

    pub fn position(&self) -> usize {
        self.position
    }

    pub fn reset(&mut self, position: usize) {
        self.position = position;
    }

    pub fn remainder(&self) -> &'t str {
        &self.text[self.position..]
    }

    pub fn peek(&self) -> Option<char> {
        self.remainder().chars().next()
    }

    pub fn peek_nth(&self, n: usize) -> Option<char> {
        self.remainder().chars().nth(n)
    }

    pub fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;

        self.position += c.len_utf8();

        Some(c)
    }

    pub fn consume(&mut self, target_char: char) -> bool {
        let found = self.peek() == Some(target_char);

        if found {
            self.position += target_char.len_utf8();
        }

        found
    }

    pub fn take_while(&mut self, mut accept: impl FnMut(char) -> bool) -> &'t str {
        let remainder = self.remainder();
        let length = remainder.find(|c| !accept(c)).unwrap_or(remainder.len());

        self.position += length;

        &remainder[..length]
    }

    pub fn skip_blank(&mut self, extended: bool) -> Result<(), Reason> {
        loop {
            if extended {
                self.take_while(is_whitespace);
                if self.peek() == Some('#') {
                    self.take_while(|c| c != '\n');
                    continue;
                }
            }

            if !self.remainder().starts_with("(?#") {
                return Ok(());
            }

            self.take_while(|c| c != ')');
            if !self.consume(')') {
                return Err(Reason::UnterminatedComment);
            }
        }
    }

    pub fn at_quantifier(&self) -> bool {
        let remainder = self.remainder().as_bytes();
        let digits = |from: usize| {
            remainder[from..]
                .iter()
                .take_while(|b| b.is_ascii_digit())
                .count()
        };

        match remainder.first() {
            Some(b'*' | b'+' | b'?') => true,
            Some(b'{') => {
                let after_min = 1 + digits(1);

                after_min > 1
                    && match remainder.get(after_min) {
                        Some(b'}') => true,
                        Some(b',') => {
                            let after_max = after_min + 1 + digits(after_min + 1);

                            remainder.get(after_max) == Some(&b'}')
                        }
                        _ => false,
                    }
            }
            _ => false,
        }
    }

    pub fn decimal(&mut self) -> Result<u32, Reason> {
        self.take_while(|c| c.is_ascii_digit())
            .bytes()
            .try_fold(0_u32, |number, digit| {
                number
                    .checked_mul(10)?
                    .checked_add(u32::from(digit - b'0'))
                    .filter(|&number| number <= MAX_NUMBER)
            })
            .ok_or(Reason::CaptureGroupOutOfRange)
    }

    pub fn name(&mut self) -> &'t str {
        self.take_while(is_word_char)
    }

    pub fn property(&mut self) -> Result<&'t str, Reason> {
        if self.peek_nth(2).is_none() {
            return Err(Reason::IncompleteSlashP);
        }

        if !self.consume('{') {
            return Err(Reason::MalformedSlashP);
        }

        let name = self.take_while(|c| is_word_char(c) || c == '-');

        if !self.consume('}') {
            return Err(Reason::IncompleteSlashP);
        }

        Ok(name)
    }

    pub fn backreference(&mut self) -> Result<Option<Reference<'t>>, Reason> {
        let close = if self.consume('k') {
            let close = match self.bump() {
                Some('<') => '>',
                Some('\'') => '\'',
                _ => return Err(Reason::MalformedNameRef),
            };

            if self.peek().is_none() {
                return Err(Reason::MalformedNameRef);
            }

            Some(close)
        } else if matches!(self.peek(), Some('<' | '\'')) && self.peek_nth(1).is_some() {
            Some(if self.bump() == Some('<') { '>' } else { '\'' })
        } else {
            None
        };

        Ok(match (close, self.peek()) {
            (Some(close), Some(c)) if c.is_ascii_digit() => {
                let number = self.decimal()?;

                self.consume(close).then_some(Reference::Number(number))
            }
            (None, Some('1'..='9')) => Some(Reference::Plain(self.decimal()?)),
            (Some(close), Some(c)) if is_word_char(c) => {
                let name = self.name();

                self.consume(close).then_some(Reference::Name(name))
            }
            _ => None,
        })
    }

    pub fn char_escape(&mut self) -> Result<char, Reason> {
        let c = self.bump().ok_or(Reason::IllegalEndEscape)?;

        Ok(match c {
            '0'..='7' => self.octal(c),
            'x' => self.hex(2)?,
            'u' => self.hex(4)?,
            'a' => '\x07',
            'b' => '\x08',
            'e' => '\x1b',
            'f' => '\x0c',
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            'v' => '\x0b',
            'c' => self.control()?,
            c if is_word_char(c) => return Err(Reason::UnrecognizedEscape(c)),
            c => c,
        })
    }

    fn octal(&mut self, first: char) -> char {
        let mut value = first.to_digit(8).unwrap_or(0);

        for _ in 0..2 {
            let Some(digit) = self.peek().and_then(|c| c.to_digit(8)) else {
                break;
            };

            self.bump();
            value = value * 8 + digit;
        }

        char::from_u32(value & 0xFF).unwrap_or_default()
    }

    fn hex(&mut self, digits: usize) -> Result<char, Reason> {
        let unit = self.hex_unit(digits).ok_or(Reason::TooFewHex)?;

        if let Some(c) = char::from_u32(unit) {
            return Ok(c);
        }

        if (0xD800..0xDC00).contains(&unit) && self.remainder().starts_with("\\u") {
            self.position += 2;
            if let Some(low) = self
                .hex_unit(4)
                .filter(|low| (0xDC00..0xE000).contains(low))
            {
                let c = 0x10000 + ((unit - 0xD800) << 10) + (low - 0xDC00);

                return char::from_u32(c).ok_or(Reason::LoneSurrogate);
            }
        }

        Err(Reason::LoneSurrogate)
    }

    fn hex_unit(&mut self, digits: usize) -> Option<u32> {
        let hex = self.remainder().get(..digits)?;

        if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }

        self.position += digits;

        u32::from_str_radix(hex, 16).ok()
    }

    fn control(&mut self) -> Result<char, Reason> {
        let c = self.bump().ok_or(Reason::MissingControl)?;

        u32::from(c.to_ascii_uppercase())
            .checked_sub(u32::from('@'))
            .filter(|&code| code < 0x20)
            .and_then(char::from_u32)
            .ok_or(Reason::UnrecognizedControl)
    }
}
