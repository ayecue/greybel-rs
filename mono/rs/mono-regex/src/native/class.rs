use super::ast::{Class, ClassItem, Set, SetKind};
use super::cursor::Cursor;
use super::error::Reason;

pub fn parse(cursor: &mut Cursor<'_>) -> Result<Class, Reason> {
    scan(cursor, true)
}

pub fn skip(cursor: &mut Cursor<'_>) -> Result<(), Reason> {
    scan(cursor, false).map(drop)
}

pub fn set_escape(cursor: &mut Cursor<'_>) -> Result<Option<Set>, Reason> {
    let Some(letter) = cursor.peek().filter(|&c| is_set_letter(c)) else {
        return Ok(None);
    };

    cursor.bump();

    let kind = match letter.to_ascii_lowercase() {
        'd' => SetKind::Digit,
        's' => SetKind::Whitespace,
        'w' => SetKind::Word,
        _ => SetKind::Property(cursor.property()?.to_owned()),
    };

    Ok(Some(Set {
        kind,
        negated: letter.is_ascii_uppercase(),
    }))
}

fn is_set_letter(c: char) -> bool {
    matches!(c, 'd' | 'D' | 's' | 'S' | 'w' | 'W' | 'p' | 'P')
}

enum Member {
    Set(Set),
    Dash,
    Char { c: char, escaped: bool },
}

fn scan(cursor: &mut Cursor<'_>, strict: bool) -> Result<Class, Reason> {
    let mut class = Class {
        negated: cursor.consume('^'),
        items: Vec::new(),
        subtraction: None,
    };
    let mut range_start = None;
    let mut at_start = true;

    loop {
        let first = std::mem::replace(&mut at_start, false);
        let c = cursor.bump().ok_or(Reason::UnterminatedBracket)?;

        if c == ']' && !first {
            return Ok(class);
        }

        let (c, escaped) = match member(cursor, c, range_start.is_some(), strict)? {
            Member::Set(set) => {
                class.items.push(ClassItem::Set(set));
                continue;
            }
            Member::Dash => {
                class.items.push(ClassItem::Char('-'));
                continue;
            }
            Member::Char { c, escaped } => (c, escaped),
        };

        if let Some(start) = range_start.take() {
            if !strict {
                continue;
            }

            if c == '[' && !escaped {
                class.items.push(ClassItem::Char(start));
                class.subtraction = Some(subtraction(cursor)?);
            } else if start > c {
                return Err(Reason::ReversedCharRange);
            } else {
                class.items.push(ClassItem::Range(start, c));
            }
        } else if cursor.peek() == Some('-') && cursor.peek_nth(1).is_some_and(|c| c != ']') {
            range_start = Some(c);
            cursor.bump();
        } else if c == '-' && !escaped && !first && cursor.peek() == Some('[') {
            cursor.bump();

            if strict {
                class.subtraction = Some(subtraction(cursor)?);
            } else {
                skip(cursor)?;
            }
        } else {
            class.items.push(ClassItem::Char(c));
        }
    }
}

fn member(
    cursor: &mut Cursor<'_>,
    c: char,
    in_range: bool,
    strict: bool,
) -> Result<Member, Reason> {
    match c {
        '\\' if cursor.peek().is_some() => {
            let checked = strict && in_range;

            if let Some(letter) = cursor.peek().filter(|&c| checked && is_set_letter(c)) {
                return Err(Reason::BadClassInCharRange(letter));
            }

            if let Some(set) = set_escape(cursor)? {
                return Ok(Member::Set(set));
            }

            if cursor.consume('-') {
                return Ok(Member::Dash);
            }

            let c = cursor.char_escape()?;

            Ok(Member::Char { c, escaped: true })
        }
        '[' if !in_range && cursor.peek() == Some(':') => {
            let start = cursor.position();

            cursor.bump();
            cursor.name();
            if !(cursor.consume(':') && cursor.consume(']')) {
                cursor.reset(start);
            }

            Ok(Member::Char { c, escaped: false })
        }
        c => Ok(Member::Char { c, escaped: false }),
    }
}

fn subtraction(cursor: &mut Cursor<'_>) -> Result<Box<Class>, Reason> {
    let class = parse(cursor)?;

    if cursor.peek().is_some_and(|c| c != ']') {
        return Err(Reason::SubtractionMustBeLast);
    }

    Ok(Box::new(class))
}
