use std::fmt::Write as _;

use fancy_regex::Captures;

use super::cursor::{Cursor, is_word_char};
use super::error::{PatternError, Reason};
use super::groups::GroupTable;

pub struct Replacement {
    pieces: Vec<Piece>,
}

enum Piece {
    Template(String),
    Before,
    After,
}

enum Substitution {
    Dollar,
    Group(usize),
    Before,
    After,
    Input,
}

impl Replacement {
    pub fn new(replacement: &str, groups: &GroupTable, input: &str) -> Result<Self, PatternError> {
        let mut cursor = Cursor::new(replacement);
        let mut pieces = Vec::new();
        let mut template = String::new();

        loop {
            push_text(&mut template, cursor.take_while(|c| c != '$'));
            if !cursor.consume('$') {
                break;
            }

            let substitution = substitution(&mut cursor, groups)
                .map_err(|reason| PatternError::new(replacement, reason))?;
            let piece = match substitution {
                None | Some(Substitution::Dollar) => {
                    template.push_str("$$");
                    continue;
                }
                Some(Substitution::Group(index)) => {
                    let _ = write!(template, "${{{index}}}");

                    continue;
                }
                Some(Substitution::Input) => {
                    push_text(&mut template, input);
                    continue;
                }
                Some(Substitution::Before) => Piece::Before,
                Some(Substitution::After) => Piece::After,
            };

            if !template.is_empty() {
                pieces.push(Piece::Template(std::mem::take(&mut template)));
            }

            pieces.push(piece);
        }

        if !template.is_empty() {
            pieces.push(Piece::Template(template));
        }

        Ok(Self { pieces })
    }

    #[must_use]
    pub fn template(&self) -> Option<&str> {
        match self.pieces.as_slice() {
            [] => Some(""),
            [Piece::Template(template)] => Some(template),
            _ => None,
        }
    }

    #[must_use]
    pub fn expand(&self, captures: &Captures<'_>, input: &str) -> String {
        let full_match = captures.get(0).expect("group 0 is the match");
        let mut out = String::new();

        for piece in &self.pieces {
            match piece {
                Piece::Template(template) => captures.expand(template, &mut out),
                Piece::Before => out.push_str(&input[..full_match.start()]),
                Piece::After => out.push_str(&input[full_match.end()..]),
            }
        }

        out
    }
}

fn push_text(template: &mut String, text: &str) {
    template.push_str(&text.replace('$', "$$"));
}

fn substitution(
    cursor: &mut Cursor<'_>,
    groups: &GroupTable,
) -> Result<Option<Substitution>, Reason> {
    let start = cursor.position();
    let braced = cursor.peek() == Some('{') && cursor.peek_nth(1).is_some();

    if braced {
        cursor.bump();
    }

    let substitution = match cursor.peek() {
        Some(c) if c.is_ascii_digit() => {
            let number = cursor.decimal()?;

            (!braced || cursor.consume('}'))
                .then(|| groups.index(number))
                .flatten()
                .map(Substitution::Group)
        }
        Some(c) if braced && is_word_char(c) => {
            let name = cursor.name();

            cursor
                .consume('}')
                .then(|| groups.number(name).and_then(|number| groups.index(number)))
                .flatten()
                .map(Substitution::Group)
        }
        Some(c) if !braced => {
            let substitution = match c {
                '$' => Some(Substitution::Dollar),
                '&' => Some(Substitution::Group(0)),
                '`' => Some(Substitution::Before),
                '\'' => Some(Substitution::After),
                '+' => groups.index(groups.last()).map(Substitution::Group),
                '_' => Some(Substitution::Input),
                _ => None,
            };

            if substitution.is_some() {
                cursor.bump();
            }

            substitution
        }
        _ => None,
    };

    if substitution.is_none() {
        cursor.reset(start);
    }

    Ok(substitution)
}

#[cfg(test)]
#[path = "tests/replacement.rs"]
mod tests;
