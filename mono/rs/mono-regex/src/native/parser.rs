//! Parses and validates .NET patterns into a [`Node`] tree.

use super::ast::{Assertion, Condition, GroupKind, Matcher, Node};
use super::class;
use super::cursor::{Cursor, Reference, is_word_char};
use super::error::Reason;
use super::groups::SlotTable;
use super::options::Options;

pub fn parse(pattern: &str, options: Options, slots: &SlotTable) -> Result<Node, Reason> {
    let mut parser = Parser {
        cursor: Cursor::new(pattern),
        options,
        slots,
        autocap: 1,
        condition_next: false,
        in_test: false,
    };
    let node = parser.alternation()?;

    match parser.cursor.peek() {
        None => Ok(node),
        Some(_) => Err(Reason::TooManyParens),
    }
}

enum Open {
    Group(GroupKind),
    Conditional(Condition),
    Options,
}

struct Parser<'p, 's> {
    cursor: Cursor<'p>,
    options: Options,
    slots: &'s SlotTable,
    autocap: u32,
    condition_next: bool,
    in_test: bool,
}

impl Parser<'_, '_> {
    fn alternation(&mut self) -> Result<Node, Reason> {
        self.branches().map(Node::alternation)
    }

    fn branches(&mut self) -> Result<Vec<Node>, Reason> {
        let mut branches = vec![self.branch()?];

        while self.cursor.consume('|') {
            branches.push(self.branch()?);
        }

        Ok(branches)
    }

    fn group_body(&mut self) -> Result<Vec<Node>, Reason> {
        let branches = self.branches()?;

        if !self.cursor.consume(')') {
            return Err(Reason::NotEnoughParens);
        }

        Ok(branches)
    }

    fn branch(&mut self) -> Result<Node, Reason> {
        let mut units = Vec::new();
        let mut after_quantifier = false;

        loop {
            self.skip_blank()?;

            let Some(c) = self.cursor.peek() else { break };

            if matches!(c, '|' | ')') {
                break;
            }

            if self.cursor.at_quantifier() {
                return Err(if after_quantifier {
                    Reason::NestedQuantify(c)
                } else {
                    Reason::QuantifyAfterNothing
                });
            }

            self.cursor.bump();

            let Some(unit) = self.unit(c)? else {
                after_quantifier = false;
                continue;
            };
            let (unit, apply_quantifier) = self.apply_quantifier(unit)?;

            units.push(unit);
            after_quantifier = apply_quantifier;
        }

        Ok(Node::concat(units))
    }

    fn skip_blank(&mut self) -> Result<(), Reason> {
        self.cursor.skip_blank(self.options.ignore_whitespace)
    }

    fn unit(&mut self, c: char) -> Result<Option<Node>, Reason> {
        let multiline = self.options.multiline;

        Ok(Some(match c {
            '(' => return self.group(),
            '[' => {
                let class = class::parse(&mut self.cursor)?;

                self.match_node(Matcher::Class(class))
            }
            '\\' => self.backslash()?,
            '^' if multiline => Node::Assertion(Assertion::LineStart),
            '^' => Node::Assertion(Assertion::Start),
            '$' if multiline => Node::Assertion(Assertion::LineEnd),
            '$' => Node::Assertion(Assertion::EndOrFinalNewline),
            '.' => self.match_node(Matcher::Any {
                newline: self.options.singleline,
            }),
            c => self.match_node(Matcher::Char(c)),
        }))
    }

    fn match_node(&self, matcher: Matcher) -> Node {
        Node::Match {
            matcher,
            ignore_case: self.options.ignore_case,
        }
    }

    fn apply_quantifier(&mut self, unit: Node) -> Result<(Node, bool), Reason> {
        self.skip_blank()?;
        if !self.cursor.at_quantifier() {
            return Ok((unit, false));
        }

        let (min, max) = match self.cursor.bump() {
            Some('*') => (0, None),
            Some('+') => (1, None),
            Some('?') => (0, Some(1)),
            _ => self.bounds()?,
        };

        self.skip_blank()?;

        let lazy = self.cursor.consume('?');

        if max.is_some_and(|max| min > max) {
            return Err(Reason::IllegalRange);
        }

        let node = Node::Repeat {
            node: Box::new(unit),
            min,
            max,
            lazy,
        };

        Ok((node, true))
    }

    fn bounds(&mut self) -> Result<(u32, Option<u32>), Reason> {
        let min = self.cursor.decimal()?;
        let max = if !self.cursor.consume(',') {
            Some(min)
        } else if self.cursor.peek() == Some('}') {
            None
        } else {
            Some(self.cursor.decimal()?)
        };

        self.cursor.bump();

        Ok((min, max))
    }

    fn group(&mut self) -> Result<Option<Node>, Reason> {
        let (outer_options, in_test) = (self.options, self.in_test);
        let node = match self.group_open()? {
            Open::Options => return Ok(None),
            Open::Group(kind) => {
                self.in_test = false;

                Node::Group {
                    kind,
                    body: Box::new(Node::alternation(self.group_body()?)),
                }
            }
            Open::Conditional(condition) => {
                self.in_test = matches!(condition, Condition::Expression(_));

                let mut branches = self.group_body()?.into_iter();
                let (Some(yes), no, None) = (branches.next(), branches.next(), branches.next())
                else {
                    return Err(Reason::TooManyAlternates);
                };

                Node::Conditional {
                    condition,
                    yes: Box::new(yes),
                    no: no.map(Box::new),
                }
            }
        };

        (self.options, self.in_test) = (outer_options, in_test);

        Ok(Some(node))
    }

    fn group_open(&mut self) -> Result<Open, Reason> {
        let condition = std::mem::take(&mut self.condition_next);

        if self.cursor.peek() != Some('?') || self.cursor.remainder().starts_with("?)") {
            let kind = if self.options.explicit_capture || condition {
                GroupKind::NonCapture
            } else {
                let number = self.autocap;

                self.autocap += 1;

                GroupKind::Capture(number)
            };

            return Ok(Open::Group(kind));
        }

        self.cursor.bump();

        let after_question = self.cursor.position();
        let kind = match self.cursor.bump() {
            Some(':') => GroupKind::NonCapture,
            Some('=') => GroupKind::Lookahead,
            Some('!') => GroupKind::NegativeLookahead,
            Some('>') => GroupKind::Atomic,
            Some(open @ ('<' | '\'')) => self.named_group(open)?,
            Some('(') => return self.condition().map(Open::Conditional),
            Some(_) => {
                self.cursor.reset(after_question);

                return self.inline_options();
            }
            None => return Err(Reason::UnrecognizedGroup),
        };

        Ok(Open::Group(kind))
    }

    fn inline_options(&mut self) -> Result<Open, Reason> {
        if !self.in_test {
            self.options.scan(&mut self.cursor);
        }

        match self.cursor.bump() {
            Some(')') => Ok(Open::Options),
            Some(':') => Ok(Open::Group(GroupKind::NonCapture)),
            _ => Err(Reason::UnrecognizedGroup),
        }
    }

    fn named_group(&mut self, open: char) -> Result<GroupKind, Reason> {
        let close = if open == '<' { '>' } else { '\'' };

        match self.cursor.peek() {
            None => return Err(Reason::UnrecognizedGroup),
            Some('=' | '!') if open == '\'' => return Err(Reason::UnrecognizedGroup),
            Some('=') => {
                self.cursor.bump();

                return Ok(GroupKind::Lookbehind);
            }
            Some('!') => {
                self.cursor.bump();

                return Ok(GroupKind::NegativeLookbehind);
            }
            _ => {}
        }

        let dash_first = self.cursor.peek() == Some('-');
        let number = match self.cursor.peek() {
            Some(c) if c.is_ascii_digit() => {
                let number = self.cursor.decimal()?;

                self.expect_name_end(close, true)?;
                if number == 0 {
                    return Err(Reason::CapnumNotZero);
                }

                self.slots.contains(number).then_some(number)
            }
            Some(c) if is_word_char(c) => {
                let name = self.cursor.name();

                self.expect_name_end(close, true)?;

                self.slots.number(name)
            }
            Some('-') => None,
            _ => return Err(Reason::InvalidGroupName),
        };
        let balanced = (number.is_some() || dash_first)
            && self.cursor.peek_nth(1).is_some()
            && self.cursor.consume('-');

        if balanced {
            self.balanced_group(close)?;
        }

        let closed = self.cursor.consume(close);

        match number {
            _ if closed && balanced => Err(Reason::BalancingGroup),
            Some(number) if closed => Ok(GroupKind::Capture(number)),
            _ => Err(Reason::UnrecognizedGroup),
        }
    }

    fn balanced_group(&mut self, close: char) -> Result<u32, Reason> {
        let number = match self.cursor.peek() {
            Some(c) if c.is_ascii_digit() => {
                let number = self.cursor.decimal()?;

                if !self.slots.contains(number) {
                    return Err(Reason::UndefinedBackref(number));
                }

                number
            }
            Some(c) if is_word_char(c) => {
                let name = self.cursor.name();

                self.slots
                    .number(name)
                    .ok_or_else(|| Reason::UndefinedNameRef(name.to_owned()))?
            }
            _ => return Err(Reason::InvalidGroupName),
        };

        self.expect_name_end(close, false)?;

        Ok(number)
    }

    fn expect_name_end(&self, close: char, allow_dash: bool) -> Result<(), Reason> {
        match self.cursor.peek() {
            Some(c) if c != close && !(allow_dash && c == '-') => Err(Reason::InvalidGroupName),
            _ => Ok(()),
        }
    }

    fn condition(&mut self) -> Result<Condition, Reason> {
        let paren = self.cursor.position() - 1;

        match self.cursor.peek() {
            Some(c) if c.is_ascii_digit() => {
                let number = self.cursor.decimal()?;

                if !self.cursor.consume(')') {
                    return Err(Reason::MalformedReference(number));
                }

                if !self.slots.contains(number) {
                    return Err(Reason::UndefinedReference(number));
                }

                return Ok(Condition::Group(number));
            }
            Some(c) if is_word_char(c) => {
                let name = self.cursor.name();

                if let Some(number) = self.slots.number(name)
                    && self.cursor.consume(')')
                {
                    return Ok(Condition::Group(number));
                }
            }
            _ => {}
        }

        self.cursor.reset(paren);

        let mut group = self.cursor.remainder().chars().skip(1);

        if group.next() == Some('?') {
            match (group.next(), group.next()) {
                (Some('#'), _) => return Err(Reason::AlternationCantHaveComment),
                (Some('\''), _) => return Err(Reason::AlternationCantCapture),
                (Some('<'), Some(c)) if c != '!' && c != '=' => {
                    return Err(Reason::AlternationCantCapture);
                }
                _ => {}
            }
        }

        self.cursor.bump();
        self.condition_next = true;
        self.in_test = true;

        let group = self.group()?.ok_or(Reason::UnrecognizedGroup)?;

        Ok(Condition::Expression(Box::new(group)))
    }

    fn backslash(&mut self) -> Result<Node, Reason> {
        let c = self.cursor.peek().ok_or(Reason::IllegalEndEscape)?;

        if let Some(assertion) = escaped_assertion(c) {
            self.cursor.bump();

            return Ok(Node::Assertion(assertion));
        }

        if let Some(set) = class::set_escape(&mut self.cursor)? {
            return Ok(self.match_node(Matcher::Set(set)));
        }

        let start = self.cursor.position();
        let slots = self.slots;
        let number = match self.cursor.backreference()? {
            Some(Reference::Number(number) | Reference::Plain(number))
                if slots.contains(number) =>
            {
                Some(number)
            }
            Some(Reference::Number(number)) => return Err(Reason::UndefinedBackref(number)),
            Some(Reference::Plain(number)) if number <= 9 => {
                return Err(Reason::UndefinedBackref(number));
            }
            Some(Reference::Name(name)) => Some(
                slots
                    .number(name)
                    .ok_or_else(|| Reason::UndefinedNameRef(name.to_owned()))?,
            ),
            Some(Reference::Plain(_)) | None => None,
        };
        let matcher = match number {
            Some(number) => Matcher::Backreference(number),
            None => {
                self.cursor.reset(start);

                Matcher::Char(self.cursor.char_escape()?)
            }
        };

        Ok(self.match_node(matcher))
    }
}

fn escaped_assertion(c: char) -> Option<Assertion> {
    Some(match c {
        'b' => Assertion::Boundary,
        'B' => Assertion::NonBoundary,
        'A' => Assertion::Start,
        'G' => Assertion::Continue,
        'Z' => Assertion::EndOrFinalNewline,
        'z' => Assertion::End,
        _ => return None,
    })
}
